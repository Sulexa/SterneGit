use sternegit_reader_protocol::{Failure, Fix, FixInput, MergeInput, Merged, Model, ReadInput, SideFilesInput, Warning};

pub trait Reader {
    fn read(&self, input: ReadInput) -> Result<Model, Failure>;

    fn side_files(&self, _input: SideFilesInput) -> Result<Vec<Warning>, Failure> {
        Ok(Vec::new())
    }

    fn fix(&self, _input: FixInput) -> Result<Fix, Failure> {
        Err(Failure::Unsupported)
    }

    fn merge(&self, _input: MergeInput) -> Result<Merged, Failure> {
        Err(Failure::Unsupported)
    }
}
