use std::path::PathBuf;

use rfd::FileDialog;

use crate::models::shared::TrackIdentifier;

// A function to run a blocking dialog on a separate thread
pub async fn pick_file(
    track_id: TrackIdentifier,
    start_dir: &str,
) -> (TrackIdentifier, Option<PathBuf>) {
    let path = FileDialog::new()
        .set_directory(start_dir)
        .set_title("Choose a file to load")
        .add_filter("Soundfont Files", &["sf2"])
        .add_filter("All Files", &["*"])
        .pick_file(); // This is the blocking call
    (track_id, path)
}
