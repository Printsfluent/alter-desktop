// Minimal Telegram Bot API client (long polling, file download, video upload).
use reqwest::blocking::{multipart, Client};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

pub struct Telegram {
    client: Client,
    api: String,
    files: String,
}

impl Telegram {
    pub fn new(token: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("http client");
        Self {
            client,
            api: format!("https://api.telegram.org/bot{token}"),
            files: format!("https://api.telegram.org/file/bot{token}"),
        }
    }

    fn call(&self, method: &str, body: Value) -> Result<Value, String> {
        let resp: Value = self
            .client
            .post(format!("{}/{method}", self.api))
            .json(&body)
            .send()
            .map_err(|e| format!("{method}: {}", e.without_url()))?
            .json()
            .map_err(|e| format!("{method}: {}", e.without_url()))?;
        if resp["ok"].as_bool() == Some(true) {
            Ok(resp["result"].clone())
        } else {
            Err(resp["description"].as_str().unwrap_or("Telegram API error").to_string())
        }
    }

    pub fn me(&self) -> Result<Value, String> {
        self.call("getMe", json!({}))
    }

    pub fn delete_webhook(&self) -> Result<(), String> {
        self.call("deleteWebhook", json!({ "drop_pending_updates": false })).map(|_| ())
    }

    pub fn configure_command_menu(&self) -> Result<(), String> {
        self.call("setMyCommands", json!({
            "commands": [
                { "command": "start", "description": "Start the bot" },
                { "command": "help", "description": "Show help" },
                { "command": "id", "description": "Show your Telegram ID" },
                { "command": "cancel", "description": "Cancel a pending upload" }
            ]
        }))?;
        self.call("setChatMenuButton", json!({
            "menu_button": { "type": "commands" }
        }))?;
        Ok(())
    }

    pub fn updates(&self, offset: i64) -> Result<Vec<Value>, String> {
        let res = self.call(
            "getUpdates",
            json!({ "offset": offset, "timeout": 50, "allowed_updates": ["message"] }),
        )?;
        Ok(res.as_array().cloned().unwrap_or_default())
    }

    pub fn send_text(&self, chat: i64, reply_to: Option<i64>, text: &str) {
        let _ = self.send_text_with_id(chat, reply_to, text);
    }

    pub fn send_text_with_id(&self, chat: i64, reply_to: Option<i64>, text: &str) -> Option<i64> {
        let mut body = json!({ "chat_id": chat, "text": text });
        if let Some(id) = reply_to {
            body["reply_parameters"] = json!({ "message_id": id, "allow_sending_without_reply": true });
        }
        match self.call("sendMessage", body) {
            Ok(result) => result["message_id"].as_i64(),
            Err(e) => {
                eprintln!("sendMessage to {chat} failed: {e}");
                None
            }
        }
    }

    pub fn edit_text(&self, chat: i64, message_id: i64, text: &str) {
        if let Err(e) = self.call("editMessageText", json!({
            "chat_id": chat,
            "message_id": message_id,
            "text": text
        })) {
            eprintln!("editMessageText in chat {chat} failed: {e}");
        }
    }

    pub fn typing_video(&self, chat: i64) {
        let _ = self.call("sendChatAction", json!({ "chat_id": chat, "action": "upload_video" }));
    }

    pub fn download(&self, file_id: &str, dest: &Path) -> Result<(), String> {
        let file = self.call("getFile", json!({ "file_id": file_id }))?;
        let remote = file["file_path"].as_str().ok_or("Telegram did not return a file path")?;
        let mut resp = self
            .client
            .get(format!("{}/{remote}", self.files))
            .send()
            .map_err(|e| format!("download: {}", e.without_url()))?;
        if !resp.status().is_success() {
            return Err(format!("download failed with HTTP {}", resp.status()));
        }
        let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        resp.copy_to(&mut out).map_err(|e| format!("download: {}", e.without_url()))?;
        Ok(())
    }

    pub fn send_video(&self, chat: i64, reply_to: i64, path: &Path, caption: &str) -> Result<(), String> {
        let form = multipart::Form::new()
            .text("chat_id", chat.to_string())
            .text("caption", caption.to_string())
            .text("supports_streaming", "true")
            .text(
                "reply_parameters",
                json!({ "message_id": reply_to, "allow_sending_without_reply": true }).to_string(),
            )
            .file("video", path)
            .map_err(|e| e.to_string())?;
        let resp: Value = self
            .client
            .post(format!("{}/sendVideo", self.api))
            .multipart(form)
            .send()
            .map_err(|e| format!("sendVideo: {}", e.without_url()))?
            .json()
            .map_err(|e| format!("sendVideo: {}", e.without_url()))?;
        if resp["ok"].as_bool() == Some(true) {
            Ok(())
        } else {
            Err(resp["description"].as_str().unwrap_or("upload failed").to_string())
        }
    }

    pub fn send_document(&self, chat: i64, reply_to: i64, path: &Path, caption: &str) -> Result<(), String> {
        let form = multipart::Form::new()
            .text("chat_id", chat.to_string())
            .text("caption", caption.to_string())
            .text("reply_parameters", json!({ "message_id": reply_to, "allow_sending_without_reply": true }).to_string())
            .file("document", path)
            .map_err(|e| e.to_string())?;
        let resp: Value = self.client.post(format!("{}/sendDocument", self.api)).multipart(form).send()
            .map_err(|e| format!("sendDocument: {}", e.without_url()))?.json()
            .map_err(|e| format!("sendDocument: {}", e.without_url()))?;
        if resp["ok"].as_bool() == Some(true) { Ok(()) }
        else { Err(resp["description"].as_str().unwrap_or("upload failed").to_string()) }
    }
}
