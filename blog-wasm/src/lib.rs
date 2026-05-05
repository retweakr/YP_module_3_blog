//! WASM-фронт по HTTP к API блога (в браузере обычно не используют gRPC — только REST).
//!
//! [`BlogApp`] держит JWT в памяти и дублирует его в `localStorage` под ключом `blog_token`.

use std::cell::RefCell;
use std::rc::Rc;

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const STORAGE_KEY: &str = "blog_token";

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDto {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostDto {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct RegisterBody<'a> {
    username: &'a str,
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct LoginBody<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
struct CreatePostBody<'a> {
    title: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct UpdatePostBody<'a> {
    title: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListPostsBody {
    posts: Vec<PostDto>,
    total: i64,
    limit: i64,
    offset: i64,
}

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn js_sys_err(_: JsValue) -> JsValue {
    JsValue::from_str("browser storage operation failed")
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no window")
}

fn storage() -> Result<web_sys::Storage, JsValue> {
    window()
        .local_storage()
        .map_err(|_| js_err("localStorage unavailable"))?
        .ok_or_else(|| js_err("localStorage disabled"))
}

fn save_token_to_storage(token: &str) -> Result<(), JsValue> {
    storage()?.set_item(STORAGE_KEY, token).map_err(js_sys_err)
}

fn get_token_from_storage() -> Result<Option<String>, JsValue> {
    storage()?
        .get_item(STORAGE_KEY)
        .map_err(js_sys_err)
}

fn clear_token_storage() -> Result<(), JsValue> {
    storage()?.remove_item(STORAGE_KEY).map_err(js_sys_err)
}

#[wasm_bindgen]
pub struct BlogApp {
    api_base: String,
    /// Состояние токена (JS однопоточный — достаточно `RefCell`).
    token: Rc<RefCell<Option<String>>>,
}

#[wasm_bindgen]
impl BlogApp {
    /// Базовый URL API, например `http://localhost:8080` (origin сервера blog-server).
    #[wasm_bindgen(constructor)]
    pub fn new(api_base: String) -> BlogApp {
        let token = Rc::new(RefCell::new(None));
        if let Ok(Some(t)) = get_token_from_storage() {
            if !t.is_empty() {
                *token.borrow_mut() = Some(t);
            }
        }
        BlogApp {
            api_base: api_base.trim_end_matches('/').to_string(),
            token,
        }
    }

    #[wasm_bindgen]
    pub fn api_base(&self) -> String {
        self.api_base.clone()
    }

    #[wasm_bindgen]
    pub fn is_authenticated(&self) -> bool {
        self.token.borrow().is_some()
    }

    #[wasm_bindgen]
    pub fn logout(&self) -> Result<(), JsValue> {
        *self.token.borrow_mut() = None;
        clear_token_storage()
    }

    #[wasm_bindgen]
    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<JsValue, JsValue> {
        if username.trim().is_empty() || email.trim().is_empty() || password.is_empty() {
            return Err(js_err("All fields are required"));
        }
        let url = format!("{}/api/auth/register", self.api_base);
        let res = Request::post(&url)
            .json(&RegisterBody {
                username: &username,
                email: &email,
                password: &password,
            })
            .map_err(js_err)?
            .send()
            .await
            .map_err(js_err)?;
        if res.status() == 409 {
            return Err(js_err("User already exists"));
        }
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        let auth: AuthResponse = res.json().await.map_err(js_err)?;
        *self.token.borrow_mut() = Some(auth.token.clone());
        save_token_to_storage(&auth.token)?;
        serde_wasm_bindgen::to_value(&auth).map_err(js_err)
    }

    #[wasm_bindgen]
    pub async fn login(&self, username: String, password: String) -> Result<JsValue, JsValue> {
        if username.trim().is_empty() || password.is_empty() {
            return Err(js_err("Username and password are required"));
        }
        let url = format!("{}/api/auth/login", self.api_base);
        let res = Request::post(&url)
            .json(&LoginBody {
                username: &username,
                password: &password,
            })
            .map_err(js_err)?
            .send()
            .await
            .map_err(js_err)?;
        if res.status() == 401 {
            return Err(js_err("Invalid credentials"));
        }
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        let auth: AuthResponse = res.json().await.map_err(js_err)?;
        *self.token.borrow_mut() = Some(auth.token.clone());
        save_token_to_storage(&auth.token)?;
        serde_wasm_bindgen::to_value(&auth).map_err(js_err)
    }

    #[wasm_bindgen]
    pub async fn load_posts(&self) -> Result<JsValue, JsValue> {
        let url = format!("{}/api/posts?limit=50&offset=0", self.api_base);
        let res = Request::get(&url).send().await.map_err(js_err)?;
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        let body: ListPostsBody = res.json().await.map_err(js_err)?;
        serde_wasm_bindgen::to_value(&body).map_err(js_err)
    }

    #[wasm_bindgen]
    pub async fn create_post(&self, title: String, content: String) -> Result<JsValue, JsValue> {
        self.ensure_token()?;
        if title.trim().is_empty() || content.trim().is_empty() {
            return Err(js_err("Title and content are required"));
        }
        let url = format!("{}/api/posts", self.api_base);
        let tok = self.token.borrow().clone().unwrap();
        let res = Request::post(&url)
            .header("Authorization", &format!("Bearer {}", tok))
            .json(&CreatePostBody {
                title: &title,
                content: &content,
            })
            .map_err(js_err)?
            .send()
            .await
            .map_err(js_err)?;
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        let post: PostDto = res.json().await.map_err(js_err)?;
        serde_wasm_bindgen::to_value(&post).map_err(js_err)
    }

    #[wasm_bindgen]
    pub async fn update_post(
        &self,
        id: i64,
        title: String,
        content: String,
    ) -> Result<JsValue, JsValue> {
        self.ensure_token()?;
        if title.trim().is_empty() || content.trim().is_empty() {
            return Err(js_err("Title and content are required"));
        }
        let url = format!("{}/api/posts/{}", self.api_base, id);
        let tok = self.token.borrow().clone().unwrap();
        let res = Request::put(&url)
            .header("Authorization", &format!("Bearer {}", tok))
            .json(&UpdatePostBody {
                title: &title,
                content: &content,
            })
            .map_err(js_err)?
            .send()
            .await
            .map_err(js_err)?;
        if res.status() == 403 {
            return Err(js_err("Forbidden (not the author)"));
        }
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        let post: PostDto = res.json().await.map_err(js_err)?;
        serde_wasm_bindgen::to_value(&post).map_err(js_err)
    }

    #[wasm_bindgen]
    pub async fn delete_post(&self, id: i64) -> Result<JsValue, JsValue> {
        self.ensure_token()?;
        let url = format!("{}/api/posts/{}", self.api_base, id);
        let tok = self.token.borrow().clone().unwrap();
        let res = Request::delete(&url)
            .header("Authorization", &format!("Bearer {}", tok))
            .send()
            .await
            .map_err(js_err)?;
        if res.status() == 403 {
            return Err(js_err("Forbidden (not the author)"));
        }
        if res.status() == 404 {
            return Err(js_err("Post not found"));
        }
        if !res.ok() {
            return Err(js_err(format!("HTTP {}", res.status())));
        }
        serde_wasm_bindgen::to_value(&serde_json::json!({ "ok": true })).map_err(js_err)
    }
}

impl BlogApp {
    fn ensure_token(&self) -> Result<(), JsValue> {
        if self.token.borrow().is_none() {
            return Err(js_err("Not authenticated"));
        }
        Ok(())
    }
}
