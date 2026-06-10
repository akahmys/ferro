use tokio::fs;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{sleep, Duration, timeout, Instant};
use crate::config::{action_dir, base_dir};
use crate::receiver::ProprioceptiveEcho;
use crate::receiver::text_processor::{VocalTextAction, handle_vocal_text, extract_tokens};
use crate::receiver::audio_processor::{VocalAudioAction, decode_and_compute_mfcc};

struct CoalesceState {
    window_end: Instant,
    text_action: Option<VocalTextAction>,
    audio_action: Option<VocalAudioAction>,
}

pub async fn start_receiver(
    feedback_tx: UnboundedSender<Vec<String>>,
    echo_tx: UnboundedSender<ProprioceptiveEcho>,
) {
    assert!(action_dir().is_absolute(), "Action directory path must be absolute");
    assert!(feedback_tx.send(vec![]).is_ok(), "Feedback sender check must succeed");
    tokio::spawn(async move {
        let (text_path, audio_path) = (action_dir().join("vocal_text.json"), action_dir().join("vocal_audio.json"));
        let log_path = base_dir().join("action_history.log");
        let (mut last_text_ts, mut last_audio_ts) = (0i64, 0i64);
        let mut state: Option<CoalesceState> = None;
        loop {
            let step = timeout(Duration::from_millis(50), async {
                if text_path.exists() {
                    if let Ok(c) = fs::read_to_string(&text_path).await {
                        if let Ok(act) = serde_json::from_str::<VocalTextAction>(&c) {
                            if act.timestamp > last_text_ts {
                                last_text_ts = act.timestamp;
                                state = Some(update_state(state.take(), Some(act), None));
                            }
                        }
                    }
                }
                if audio_path.exists() {
                    if let Ok(c) = fs::read_to_string(&audio_path).await {
                        if let Ok(act) = serde_json::from_str::<VocalAudioAction>(&c) {
                            if act.timestamp > last_audio_ts {
                                last_audio_ts = act.timestamp;
                                state = Some(update_state(state.take(), None, Some(act)));
                            }
                        }
                    }
                }
                if let Some(s) = state.take() {
                    if (s.text_action.is_some() && s.audio_action.is_some()) || Instant::now() >= s.window_end {
                        process_coalesced(s, &feedback_tx, &echo_tx, &log_path).await;
                    } else {
                        state = Some(s);
                    }
                }
                sleep(Duration::from_millis(2)).await;
            }).await;
            if step.is_err() { break; }
        }
    });
}

fn update_state(s: Option<CoalesceState>, t: Option<VocalTextAction>, a: Option<VocalAudioAction>) -> CoalesceState {
    assert!(t.is_some() || a.is_some(), "Must provide text or audio action");
    let mut state = s.unwrap_or_else(|| CoalesceState {
        window_end: Instant::now() + Duration::from_millis(10),
        text_action: None,
        audio_action: None,
    });
    if let Some(act) = t { state.text_action = Some(act); }
    if let Some(act) = a { state.audio_action = Some(act); }
    assert!(state.text_action.is_some() || state.audio_action.is_some(), "State must hold action");
    state
}

async fn process_coalesced(
    s: CoalesceState,
    feedback_tx: &UnboundedSender<Vec<String>>,
    echo_tx: &UnboundedSender<ProprioceptiveEcho>,
    log_path: &std::path::Path,
) {
    assert!(s.text_action.is_some() || s.audio_action.is_some(), "State must have action");
    let mut tokens = Vec::new();
    let mut mfcc = vec![0.0; 5];
    if let Some(ref txt) = s.text_action {
        tokens = extract_tokens(&txt.text);
        let log_line = format!("{} [{}]: {}\n", txt.timestamp, txt.origin_cluster_id, txt.text);
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(log_path).await {
            use tokio::io::AsyncWriteExt;
            let _ = f.write_all(log_line.as_bytes()).await;
        }
        let resp_tokens = handle_vocal_text(&txt.text);
        let tx = feedback_tx.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(1500)).await;
            let _ = tx.send(resp_tokens);
        });
    }
    if let Some(ref aud) = s.audio_action {
        if let Ok(computed_mfcc) = decode_and_compute_mfcc(aud) {
            mfcc = computed_mfcc;
        }
    }
    assert_eq!(mfcc.len(), 5, "MFCC must have exactly 5 elements");
    let _ = echo_tx.send(ProprioceptiveEcho { speech_tokens: tokens, mfcc });
}
