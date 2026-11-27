use sternegit_reader_protocol::{Failure, Fix, FixInput, Model, ReadInput, SideFilesInput, Warning};

pub trait Reader {
    fn read(&self, input: ReadInput) -> Result<Model, Failure>;

    fn side_files(&self, _input: SideFilesInput) -> Result<Vec<Warning>, Failure> {
        Ok(Vec::new())
    }

    fn fix(&self, _input: FixInput) -> Result<Fix, Failure> {
        Err(Failure::Unsupported)
    }
}
