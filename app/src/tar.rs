use std::fs::metadata;
use tokio_tar::{Builder, Header};

pub async fn create_tar_archive(
    dockerfile_dir: &std::path::Path,
    tar_file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating tar archive: {}", tar_file_path);
    let tar_file = tokio::fs::File::create(tar_file_path).await?;
    println!("Created tar archive: {}", tar_file_path);
    let mut builder = Builder::new(tar_file);
    println!("Created tar builder");

    for entry in dockerfile_dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        if path.is_file() {
            let meta = metadata(&path)?;
            let mut file = tokio::fs::File::open(&path).await?;
            let mut header = Header::new_gnu();
            header.set_size(meta.len());
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, file_name, &mut file)
                .await?;
        }
    }

    builder.finish().await?;
    Ok(())
}
