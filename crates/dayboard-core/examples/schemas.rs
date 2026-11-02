fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("docs/schemas")?;
    std::fs::write(
        "docs/schemas/command.json",
        serde_json::to_string_pretty(&schemars::schema_for!(dayboard_core::model::Command))?,
    )?;
    std::fs::write(
        "docs/schemas/response.json",
        serde_json::to_string_pretty(&schemars::schema_for!(dayboard_core::model::Response))?,
    )?;
    Ok(())
}
