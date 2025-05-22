pub mod env;

pub fn load_flashrc(variables: &mut HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    let flashrc_path = format!("{}/.flashrc", home);
    
    let file = fs::File::open(&flashrc_path)?;
    let reader = BufReader::new(file);
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Process export statements
        if let Some(export_content) = line.strip_prefix("export ") {
            if let Err(e) = process_export(export_content, variables) {
                eprintln!("Warning: Invalid export on line {}: {} ({})", line_num + 1, line, e);
            }
        }
    }
    
    Ok(())
}