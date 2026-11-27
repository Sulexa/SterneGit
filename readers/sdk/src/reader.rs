use sternegit_reader_protocol::{Failure, Model, ReadInput};

pub trait Reader {
    fn read(&self, input: ReadInput) -> Result<Model, Failure>;
}
