use default_env::default_env;
use futures_util::{select, SinkExt, StreamExt};
use ng_server::protocol;
use tauri::{
    async_runtime::{channel, Mutex, Receiver, Sender},
    Manager,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::connect_async;
use url::Url;

fn process_message(message: protocol::Response, app: &tauri::AppHandle) {
    match message {
        protocol::Response::MessageFrom { sender, content } => {
            app.emit("received_message", (sender, content)).unwrap()
        }
        protocol::Response::AssignResult { excuse } => app.emit("assign_result", excuse).unwrap(),
        protocol::Response::PlayerOut {
            quitter,
            word,
            suicide,
        } => app.emit("player_out", (quitter, word, suicide)).unwrap(),
        protocol::Response::PlayerReady { name } => app.emit("player_ready", name).unwrap(),
        protocol::Response::PlayerNotReady { name } => app.emit("player_not_ready", name).unwrap(),
        protocol::Response::TimerReset { timer } => {
            app.emit("timer_reset", timer.as_secs()).unwrap()
        }
        protocol::Response::GameWin { winner, word } => {
            app.emit("game_win", (winner, word)).unwrap()
        }
        protocol::Response::GameEndTimeout => app.emit("timeout", ()).unwrap(),
        protocol::Response::GameEndUnproceedable => app.emit("unproceedable", ()).unwrap(),
        protocol::Response::StartVoteAbort => app.emit("start_vote_abort", ()).unwrap(),
        protocol::Response::VotedAbort { abort, voter } => {
            app.emit("voted_abort", (voter, abort)).unwrap()
        }
        protocol::Response::VoteAbortResult { abort } => {
            app.emit("vote_abort_result", abort).unwrap()
        }
        protocol::Response::AssignStart { assignee } => app.emit("assign_start", assignee).unwrap(),
        protocol::Response::GameStart { assigned } => app.emit("game_start", assigned).unwrap(),
        protocol::Response::Overview {
            clients,
            game_state,
        } => app.emit("overview", (clients, game_state)).unwrap(),
        protocol::Response::MessageHistory { history } => {
            app.emit("message_history", history).unwrap()
        }
        protocol::Response::PlayerJoin { name } => app.emit("player_join", name).unwrap(),
        protocol::Response::PlayerQuit { name } => app.emit("player_quit", name).unwrap(),
        protocol::Response::ReadyResult { excuse } => app.emit("ready_result", excuse).unwrap(),
        protocol::Response::LoginResult { .. } => (),
    }
}

#[tauri::command]
async fn login(
    name: &str,
    sender: tauri::State<'_, Sender<protocol::Actions>>,
    receiver: tauri::State<'_, Mutex<Receiver<protocol::Response>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    sender
        .send(protocol::Actions::Login {
            name: name.to_owned(),
        })
        .await
        .map_err(|err| err.to_string())?;

    let mut pending_messages = Vec::new();
    while let Some(response) = receiver.lock().await.recv().await {
        if let protocol::Response::LoginResult { excuse } = response {
            if let Some(excuse) = excuse {
                return Err(excuse);
            } else {
                for resp in pending_messages.into_iter() {
                    process_message(resp, &app);
                }
                return Ok(());
            }
        } else {
            pending_messages.push(response);
            continue;
        }
    }
    Err("网络异常，请重启应用".to_owned())
}

#[tauri::command]
async fn send_message(
    message: &str,
    sender: tauri::State<'_, Sender<protocol::Actions>>,
) -> Result<(), String> {
    sender
        .send(protocol::Actions::Send(message.to_string()))
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn assign_word(
    word: &str,
    sender: tauri::State<'_, Sender<protocol::Actions>>,
) -> Result<(), String> {
    sender
        .send(protocol::Actions::AssignWord {
            word: word.to_owned(),
        })
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn suicide(sender: tauri::State<'_, Sender<protocol::Actions>>) -> Result<(), String> {
    sender
        .send(protocol::Actions::Suicide)
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn request_abort(sender: tauri::State<'_, Sender<protocol::Actions>>) -> Result<(), String> {
    sender
        .send(protocol::Actions::RequestAbort)
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn set_ready(sender: tauri::State<'_, Sender<protocol::Actions>>) -> Result<(), String> {
    sender
        .send(protocol::Actions::SetReady)
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn cancel_ready(sender: tauri::State<'_, Sender<protocol::Actions>>) -> Result<(), String> {
    sender
        .send(protocol::Actions::CancelReady)
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn vote_abort(
    abort: bool,
    sender: tauri::State<'_, Sender<protocol::Actions>>,
) -> Result<(), String> {
    sender
        .send(protocol::Actions::VoteAbort { abort })
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
async fn flush_response_queue(
    receiver: tauri::State<'_, Mutex<Receiver<protocol::Response>>>,
    app: tauri::AppHandle,
) -> Result<(), ()> {
    while let Some(response) = receiver.lock().await.recv().await {
        process_message(response, &app);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (action_sender, action_receiver) = channel::<protocol::Actions>(100);
    let (response_sender, response_receiver) = channel::<protocol::Response>(100);

    tauri::Builder::default()
        .manage(action_sender)
        .manage(Mutex::new(response_receiver))
        .invoke_handler(tauri::generate_handler![login, send_message, assign_word, suicide, request_abort, set_ready, cancel_ready, vote_abort, flush_response_queue])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (socket, _) = connect_async(Url::parse(default_env!("NG_SERVER_URL", "ws://127.0.0.1:1453/")).unwrap())
                    .await
                    .unwrap();

                let (mut write, read) = socket.split();
                let mut fuse_read = read.fuse();
                let mut action_receiver_stream = ReceiverStream::new(action_receiver).fuse();

                loop {
                    select! {
                        message = fuse_read.select_next_some() => {
                            let message = message.unwrap();
                            if let tokio_tungstenite::tungstenite::Message::Text(content) = message {
                                println!("{}", content);
                                match serde_json::from_str::<protocol::Response>(&content) {
                                    Ok(response) => {
                                        let _ = response_sender.try_send(response.clone());
                                        app_handle.emit("server_event", ()).unwrap();
                                    },
                                    Err(_) => (),
                                }
                            }
                        },
                        action = action_receiver_stream.select_next_some() => {
                            write.send(tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(&action).unwrap())).await.unwrap();
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
