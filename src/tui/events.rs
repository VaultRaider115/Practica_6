use crossterm::event::{Event, EventStream, KeyEvent};
use futures::StreamExt;

use crate::app::AppError;

#[derive(Default)]
pub struct InputHandler {
    event: EventStream,
}

impl InputHandler {
    pub async fn next_event(&mut self) -> Result<Option<KeyEvent>, AppError> {
        match self.event.next().await {
            Some(Ok(event)) => match event {
                Event::Key(key) => Ok(Some(key)),
                Event::Mouse(_) => Ok(None),
                Event::Paste(_) => Ok(None),
                Event::Resize(_, _) => Ok(None),
                Event::FocusGained => Ok(None),
                Event::FocusLost => Ok(None),
            },
            Some(Err(error)) => Err(AppError::from(error)),
            None => Ok(None),
        }
    }
}
