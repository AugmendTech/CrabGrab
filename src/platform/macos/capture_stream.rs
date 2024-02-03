use std::pin::Pin;

use crate::prelude::{StreamEvent, StreamCreateError};

pub struct MacosCaptureStream {

}

impl MacosCaptureStream {
    pub fn new(_callback: Pin<Box<impl FnMut(StreamEvent) + Send>>) -> Result<Self, StreamCreateError> {
        Err(StreamCreateError::Other("MacosCaptureStream::new() unimplemented!".into()))
    }
}
