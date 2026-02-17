use rand::{distributions::Alphanumeric, Rng};

pub fn random_name() -> String {
    let prefix = "user-";
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    format!("{prefix}{suffix}")
}
