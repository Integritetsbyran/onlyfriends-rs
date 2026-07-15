pub trait Storage<T> {
    type Error;

    fn save(&mut self, obj: T) -> Result<(), Self::Error>;
}
