use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

static WHISPER_STATE: Lazy<Arc<Mutex<Option<WhisperState>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));
static WHISPER_PARAMS: Lazy<Mutex<Option<FullParams>>> = Lazy::new(|| Mutex::new(None));

pub fn init(model_path: &str) {
    whisper_rs::install_whisper_tracing_trampoline();
    // Whisper
    let ctx =
        WhisperContext::new_with_params(model_path, WhisperContextParameters::default()).unwrap();
    let state = ctx.create_state().expect("failed to create key");
    let mut params = FullParams::new(SamplingStrategy::default());

    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_language(Some("auto"));

    *WHISPER_STATE.lock().unwrap() = Some(state);
    *WHISPER_PARAMS.lock().unwrap() = Some(params);
}

pub fn transcribe(samples: &[f32]) -> Option<String> {
    let state = WHISPER_STATE.clone();
    let mut state = state.lock().unwrap();
    let state = state.as_mut().unwrap();
    let params = WHISPER_PARAMS.lock().unwrap();
    let mut params = params.clone().unwrap();

    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_language(Some("auto"));

    state.full(params, samples).unwrap();
    // Iterate through the segments of the transcript.
    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");
    let full_text = (0..num_segments)
        .map(|i| {
            // Get the transcribed text and timestamps for the current segment.
            let segment = state
                .full_get_segment_text(i)
                .expect("failed to get segment");
            let start_timestamp = state
                .full_get_segment_t0(i)
                .expect("failed to get start timestamp");
            let end_timestamp = state
                .full_get_segment_t1(i)
                .expect("failed to get end timestamp");

            let first_token_dtw_ts = if let Ok(token_count) = state.full_n_tokens(i) {
                if token_count > 0 {
                    if let Ok(token_data) = state.full_get_token_data(i, 0) {
                        token_data.t_dtw
                    } else {
                        -1i64
                    }
                } else {
                    -1i64
                }
            } else {
                -1i64
            };
            // Print the segment to stdout.
            format!(
                "[{} - {} ({})]: {}",
                start_timestamp, end_timestamp, first_token_dtw_ts, segment
            )
        })
        .collect();
    Some(full_text)
}
