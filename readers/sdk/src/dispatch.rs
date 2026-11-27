use serde::Serialize;
use sternegit_reader_protocol::{Answer, Call, Failure, Hello, PROTOCOL};

use crate::{Reader, SDK_VERSION};

pub fn dispatch(reader: &impl Reader, input: &[u8]) -> Vec<u8> {
    match serde_json::from_slice::<Call>(input) {
        Ok(Call::Hello(_)) => answer(Ok(Hello {
            protocol: PROTOCOL.to_owned(),
            sdk: Some(SDK_VERSION.to_owned()),
        })),
        Ok(Call::Read(input)) => answer(reader.read(input)),
        Ok(Call::SideFiles(input)) => answer(reader.side_files(input)),
        Ok(Call::Fix(input)) => answer(reader.fix(input)),
        Err(error) => answer::<()>(Err(Failure::Protocol {
            message: format!("SterneGit's call can't be read: {error}"),
        })),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn serve(reader: &impl Reader, input_len: i32) -> i32 {
    crate::log::report_panics();
    let input = crate::host::take(input_len as u32 as usize);
    let output = dispatch(reader, &input);
    crate::host::finish(&output);
    0
}

fn answer<T: Serialize>(result: Result<T, Failure>) -> Vec<u8> {
    serde_json::to_vec(&Answer::from(result)).unwrap_or_else(|error| {
        serde_json::to_vec(&Answer::<()>::Err(Failure::Protocol {
            message: format!("the reader's answer can't be written: {error}"),
        }))
        .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use sternegit_reader_protocol::{Answer, Failure, Hello, Model, ReadInput};

    use super::dispatch;
    use crate::Reader;

    struct Titled;

    impl Reader for Titled {
        fn read(&self, input: ReadInput) -> Result<Model, Failure> {
            Ok(Model {
                title: Some(input.path),
                ..Model::default()
            })
        }
    }

    #[test]
    fn hello_answers_the_protocol_and_sdk_versions() {
        let output = dispatch(&Titled, br#"{"op":"hello","input":{"protocol":"1.0","sdk":null}}"#);
        let hello: Answer<Hello> = serde_json::from_slice(&output).unwrap();
        assert_eq!(hello.into_result().unwrap().protocol, "1.0");
    }

    #[test]
    fn read_reaches_the_reader() {
        let output = dispatch(
            &Titled,
            br#"{"op":"read","input":{"kind":"prefab","path":"A.prefab","side":"after"}}"#,
        );
        let model: Answer<Model> = serde_json::from_slice(&output).unwrap();
        assert_eq!(model.into_result().unwrap().title.as_deref(), Some("A.prefab"));
    }

    #[test]
    fn a_call_it_cannot_read_is_answered_with_a_protocol_failure() {
        let output = dispatch(&Titled, b"{");
        let answer: Answer<()> = serde_json::from_slice(&output).unwrap();
        assert!(matches!(answer.into_result(), Err(Failure::Protocol { .. })));
    }
}
