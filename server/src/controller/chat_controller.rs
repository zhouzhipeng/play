use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use anyhow::{anyhow, Context};
use axum::{body, Form, Json};
use axum::body::{Body, BoxBody, HttpBody, StreamBody};
use axum::extract::Query;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response, Sse};
use axum::response::sse::{Event, KeepAlive};
use axum_macros::debug_handler;
use bytes::Bytes;
use crossbeam_channel::{RecvError, unbounded};
use either::Either;
use futures_core::Stream;
use futures_util::{stream, TryStreamExt};
use futures::StreamExt;
use hex::ToHex;
use http::StatusCode;
use http_body::Full;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlx::Executor;
use sqlx::{Column, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult, MySqlRow};
use tokio::fs::File;
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use tracing::{error, info};

use crate::{ensure, hex_to_string, method_router, R, string_to_hex, template};
use crate::{HTML, JSON, render_fragment, S, Template};
use crate::controller::static_controller::STATIC_DIR;
use crate::service::openai_service::{CreateMessage, Role};
use crate::tables::general_data::GeneralData;

method_router!(
    get : "/chat/test" -> test_sse,
    post : "/chat/submit" -> submit_chat,
    get : "/chat/sse-submit" -> sse_submit_chat,
    get : "/chat/threads" -> list_threads,
    get : "/chat/messages" -> list_messages_by_thread,
);

#[derive(Deserialize,Serialize, Debug)]
pub struct ChatAIReq {
    pub input: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct SSEChatAIReq {
    pub hexInput: String,
    #[serde(default)]
    pub thread_id: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct ListMessageReq {
    pub thread_id: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct ChatAIOptionReq {
    #[serde(default)]
    pub no_audio: bool,
    #[serde(default)]
    pub is_general: bool,
    #[serde(default)]
    pub thread_id: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct ChatThreadDo {
    pub thread_id: String,
    pub title: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct ChatThreadVo {
    #[serde(default)]
    pub data_id: u32,
    pub thread_id: String,
    pub title: String,
}
#[derive(Deserialize,Serialize, Debug)]
pub struct ChatMessageDo {
    pub role: String,
    pub content: String,
}


//for mygirl assistant
pub const CAT_OPENAI_THREAD: &str="openai_thread";

//for general chat.
pub  const CAT_GENERAL_OPENAI_THREADS: &str="general_openai_threads";


async fn submit_chat(s: S, Query(option): Query<ChatAIOptionReq>, Form(req): Form<ChatAIReq>) -> R<impl  IntoResponse> {
    info!("chat ai request in  >>  option : {:?},  {:?}",option,  req);
    //prepare thread id.
    let thread_id = get_thread_id(&s, &option,  &safe_substring(&req.input, 0,30)).await?;

    //create a message
    let msg = CreateMessage{ role: Role::user, content: req.input};

    //run thread.
    let ass_id = if option.is_general{&s.config.open_ai.general_assistant_id}else{&s.config.open_ai.assistant_id};
    let resp_msg = s.openai_service.run_thread_and_wait(&thread_id,ass_id,  &msg).await?;


    if option.no_audio || option.is_general{
        let  response = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain")
            .header("x-thread-id", thread_id)
            .body(Body::from(resp_msg))?;

        Ok(response)
    }else{
        // then call text to speech api
        Ok(call_tts(&s, &resp_msg).await?)
    }

}

async fn sse_submit_chat(s: S, Query(req): Query<SSEChatAIReq>) -> Sse<impl Stream<Item = Result<Event, Infallible>>>  {

    let option = ChatAIOptionReq{
        no_audio: true,
        is_general: true,
        thread_id: req.thread_id.to_string(),
    };

    let input = hex_to_string!(&req.hexInput);

    info!("sse chat ai request in  >>  {:?}", input);

    //prepare thread id.
    let thread_id = get_thread_id(&s, &option,  &safe_substring(&input, 0,30)).await.unwrap();

    //create a message
    let msg = CreateMessage{ role: Role::user, content: input};

    //run thread.
    let ass_id = s.config.open_ai.general_assistant_id.to_string();
    let (sender,  mut receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move{
       let r = s.openai_service.run_thread_sse(&thread_id,&ass_id,  &msg,sender).await;
        info!("run_thread_sse result : {:?}", r);
    });

    let stream = UnboundedReceiverStream::new(receiver)
        .map(|data| Ok(Event::default().data(string_to_hex!(data))));

    Sse::new(stream)
}
async fn test_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>>  {

    let stream = stream::unfold(0, |state| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if state<10{
            Some((Event::default().data(state.to_string()), state + 1))
        }else{
            None
        }

    })  .map(Ok);
        // .throttle(Duration::from_secs(1));;

    Sse::new(stream).keep_alive(KeepAlive::default())
}
// #[axum::debug_handler]
async fn list_threads(s: S) -> JSON<Vec<ChatThreadVo>> {
    info!("list_threads request in");
    let thread_list = GeneralData::query_latest_by_cat_with_limit(CAT_GENERAL_OPENAI_THREADS, 100,&s.db).await?;
    let return_list = thread_list.iter().map(|t|{
        let mut  dd = serde_json::from_str::<ChatThreadVo>(&t.data).unwrap();
        dd.data_id = t.id;
        dd
    }).collect();
    Ok(Json(return_list))
}

async fn list_messages_by_thread(s: S, Query(req): Query<ListMessageReq>) -> JSON<Vec<ChatMessageDo>> {
    info!("list_threads request in");
    let return_list = s.openai_service.list_messages(&req.thread_id).await?.iter().rev().map(|t|ChatMessageDo{
        role: t.role.to_string(),
        content: t.content[0].text.value.to_string(),
    }).collect();
    Ok(Json(return_list))
}

async fn call_tts(s: &S, resp_msg: &String)->R<Response<axum::body::Body>> {
    let service = &s.elevenlabs_service;
    ;
    let tts_result = service.text_to_speech(&resp_msg).await;
    return match tts_result {
        Ok(bytes_stream) => {
            // let stream_body = StreamBody::new(bytes_stream);
            ;
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("x-resp-msg", string_to_hex!(&resp_msg))
                .header("content-type", "audio/mpeg")
                .body(Body::wrap_stream(bytes_stream))?;

            Ok(response)
        }
        Err(e) => {
            error!("text_to_speech error : {} , ready to use default audio", e);
            let content = STATIC_DIR.get_file("ElevenLabs_changekey_hint.mp3").unwrap().contents();
            let body = Body::from(content);

            let response = Response::builder()
                .status(StatusCode::OK)
                .header("x-resp-msg", string_to_hex!(&resp_msg))
                .header("content-type", "audio/mpeg")
                .body(body)?;

            Ok(response)
        }
    }
}

async fn get_thread_id(s: &S, option: &ChatAIOptionReq, title: &str) -> anyhow::Result<String> {
    let thread_id = if !option.is_general {
        //use dedicated assistant.
        let openai_thread = GeneralData::query_by_cat_simple(CAT_OPENAI_THREAD,1000, &s.db).await?;
        if openai_thread.is_empty() {
            //first time , so create a new openai thread.
            info!("first time chat, creating a new chat thread...");
            let thread = s.openai_service.create_thread().await?;
            //save it
            GeneralData::insert(CAT_OPENAI_THREAD, &thread.id, &s.db).await?;
            thread.id
        } else {
            openai_thread[0].data.to_string()
        }
    } else {
        //for general chat
        if option.thread_id.is_empty() {
            //create a new thread
            info!("first time chat, creating a new chat thread...");
            let thread = s.openai_service.create_thread().await?;
            //save it
            let data = serde_json::to_string(&ChatThreadDo { thread_id: thread.id.to_string(), title: title.to_string() })?;
            GeneralData::insert(CAT_GENERAL_OPENAI_THREADS, &data, &s.db).await?;
            thread.id
        } else {
            //use existed one.
            let thread_list = GeneralData::query_by_json_field("*", CAT_GENERAL_OPENAI_THREADS, "thread_id", &option.thread_id,1, &s.db).await?;
            let openai_thread = thread_list.get(0)
                .context(format!("get thread {} error!", option.thread_id))?;
            let th_do = serde_json::from_str::<ChatThreadDo>(&openai_thread.data)?;

            //update it for order by time
            GeneralData::update_data_by_id(openai_thread.id, &openai_thread.data, &s.db).await?;

            th_do.thread_id
        }
    };
    Ok(thread_id)
}

fn safe_substring(s: &str, start_char_index: usize, end_char_index: usize) -> String {
    let start_byte_index = s.char_indices()
        .nth(start_char_index)
        .map_or_else(|| s.len(), |(index, _)| index);
    let end_byte_index = s.char_indices()
        .nth(end_char_index)
        .map_or_else(|| s.len(), |(index, _)| index);

    s[start_byte_index..end_byte_index].to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_substr() {
        let s = "用html写一段简单的聊天窗口的代码";
        // let  a= &s[0..30];
        let  a=safe_substring(s,0,300);
        println!("{:?}", a);
    }
}
