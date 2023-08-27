pub trait WrapErrors<T> {
    fn wrap_errors(self) -> Result<T, String>;
}
impl<T, E: ToString> WrapErrors<T> for Result<T, E> {
    fn wrap_errors(self) -> Result<T, String> {
        self.map_err(|err| err.to_string())
    }
}
