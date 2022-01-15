/// Helper to quit with a status code and no message.
#[derive(Fail, Debug)]
#[fail(display = "exit with status code {}", _0)]
// #[ignore(dead_code)]
pub struct QuietExit(pub i32);
