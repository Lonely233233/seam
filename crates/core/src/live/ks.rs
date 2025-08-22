use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;

use crate::{
    common::CLIENT,
    error::{Result, SeamError},
    util::{hash2header, parse_url},
};

use super::{Live, Node};

const URL: &str = "https://live.kuaishou.com/u/";

/// 快手直播
///
/// https://live.kuaishou.com/
pub struct Client;

#[async_trait]
impl Live for Client {
    // TODO 说明所需 cookie
    async fn get(&self, rid: &str, headers: Option<HashMap<String, String>>) -> Result<Node> {
        let text = CLIENT
            .get(format!("{URL}{rid}"))
            .headers(hash2header(headers))
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 Edg/117.0.2045.31")
            .send()
            .await?
            .text()
            .await?;
        // 优化正则表达式，参考 Python 脚本
        let re = Regex::new(r#"<script>window.__INITIAL_STATE__=(.*?);\(function\(\)\{var s;"#)?;
        let stream = match re.captures(&text) {
            Some(caps) => caps.get(1).ok_or(SeamError::NeedFix("stream"))?.as_str(),
            None => {
                return Err(SeamError::NeedFix("stream none"));
            }
        };
        // 解析 JSON 数据，添加调试信息
        let json: serde_json::Value = match serde_json::from_str(stream) {
            Ok(json) => json,
            Err(e) => {
                // 打印部分 stream 内容以便调试
                let snippet = if stream.len() > 100 {
                    &stream[..100]
                } else {
                    stream
                };
                return Err(SeamError::NeedFix(&format!(
                    "JSON parse error: {} at '{}'",
                    e, snippet
                )));
            }
        };

        let title = json["liveroom"]["playList"][0]["liveStream"]["caption"]
            .as_str()
            .unwrap_or("获取失败")
            .to_owned();

        let cover = json["liveroom"]["playList"][0]["liveStream"]["poster"]
            .as_str()
            .unwrap_or("")
            .to_owned();

        let head = json["liveroom"]["playList"][0]["author"]["avatar"]
            .as_str()
            .unwrap_or("")
            .to_owned();

        let anchor = json["liveroom"]["playList"][0]["author"]["name"]
            .as_str()
            .unwrap_or("获取失败")
            .to_owned();

        // 提取直播源
        let play_urls = json["liveroom"]["playList"][0]["liveStream"]
            .get("playUrls")
            .and_then(|pu| pu.as_array())
            .and_then(|arr| arr.get(0))
            .ok_or(SeamError::None)?;

        let representation = if play_urls.get("h264").is_some() {
            play_urls["h264"]["adaptationSet"]["representation"]
                .as_array()
                .ok_or(SeamError::NeedFix("h264 representation"))?
        } else {
            play_urls["adaptationSet"]["representation"]
                .as_array()
                .ok_or(SeamError::NeedFix("representation"))?
        };

        let url = representation
            .last()
            .and_then(|rep| rep["url"].as_str())
            .ok_or(SeamError::NeedFix("url"))?;
        let urls = vec![parse_url(url.to_string())];

        Ok(Node {
            rid: rid.to_owned(),
            title,
            cover,
            anchor,
            head,
            urls,
        })
    }
}

#[cfg(test)]
macros::gen_test!(Bd20210915);
