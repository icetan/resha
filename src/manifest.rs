#[derive(Debug)]
pub struct ReifyStatus {
    // pub output: String,
    pub success: bool,
}

impl ReifyStatus {
    // pub fn output(&self) -> &String {
    //     &self.output
    // }

    pub fn success(&self) -> bool {
        self.success
    }
}
