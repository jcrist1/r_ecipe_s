use base64::Engine;
use sha2::Digest;
fn main() {
    let mut hasher = sha2::Sha512::new();
    hasher.update("secret123");
    let digest = hasher.finalize();

    let str = base64::engine::general_purpose::STANDARD.encode(digest);
    println!("{}", str)
}
