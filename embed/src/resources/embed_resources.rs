use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/embedfiles/"]
struct Asset;

pub fn get_app_default() -> Option<rust_embed::EmbeddedFile> {
    Asset::get("./app_default.yml")
}
