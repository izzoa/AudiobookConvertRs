use audiobookconvert_rs::state::AppState;
use audiobookconvert_rs::models::{AudioBook, ConversionSettings, ConversionJob, JobStatus};

#[test]
fn test_audiobook_state_management_simulation() {
    let state = AppState::new();
    
    // Simulate "create_audiobook" command
    let audiobook = AudioBook::new("Integration Test Book".to_string());
    let id = audiobook.id.clone();
    
    state.add_audiobook(audiobook.clone());
    
    let current = state.get_active_audiobook();
    assert!(current.is_some());
    assert_eq!(current.unwrap().id, id);
    
    // Simulate "clear_audiobook"
    state.set_active_audiobook(None).unwrap();
    assert!(state.get_active_audiobook().is_none());
}

#[test]
fn test_conversion_job_flow_simulation() {
    let state = AppState::new();
    let audiobook = AudioBook::new("Job Test Book".to_string());
    let settings = ConversionSettings::default();
    
    // Simulate "start_conversion" command (add to queue)
    let job = ConversionJob::new(audiobook, settings);
    let job_id = job.id.clone();
    
    state.add_job(job);
    
    let queued_job = state.get_job(&job_id);
    assert!(queued_job.is_some());
    assert_eq!(queued_job.unwrap().status, JobStatus::Queued);
    
    // Simulate "cancel_conversion"
    state.cancel_job(&job_id).unwrap();
    
    let cancelled_job = state.get_job(&job_id);
    assert_eq!(cancelled_job.unwrap().status, JobStatus::Cancelled);
}

#[test]
fn test_metadata_update_simulation() {
    let state = AppState::new();
    let mut audiobook = AudioBook::new("Metadata Test".to_string());
    audiobook.author = Some("Old Author".to_string());
    state.add_audiobook(audiobook);

    // Simulate "update_metadata" logic (calling editor)
    // Here we just modify state directly as the command does via editor
    let mut current = state.get_active_audiobook().unwrap();
    current.output_metadata.artist = Some("New Author".to_string());
    current.author = Some("New Author".to_string()); // mimic editor behavior
    state.update_audiobook(current).unwrap();
    
    let updated = state.get_active_audiobook().unwrap();
    assert_eq!(updated.output_metadata.artist.as_deref(), Some("New Author"));
    assert_eq!(updated.author.as_deref(), Some("New Author"));
}
