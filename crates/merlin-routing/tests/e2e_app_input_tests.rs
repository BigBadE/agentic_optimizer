//! End-to-end tests that drive the TUI app via an injected input event source.

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use merlin_routing::user_interface::{event_source::InputEventSource, TuiApp};
    use merlin_routing::Result;
    use std::collections::VecDeque;
    use std::thread;
    use std::time::Duration;

    #[derive(Default)]
    struct TestEventSource {
        queue: VecDeque<Event>,
    }

    impl TestEventSource {
        fn with_events(events: impl IntoIterator<Item = Event>) -> Self {
            let mut source = Self::default();
            for event_item in events {
                source.queue.push_back(event_item);
            }
            source
        }
    }

    impl InputEventSource for TestEventSource {
        fn poll(&mut self, timeout: Duration) -> bool {
            if !self.queue.is_empty() {
                return true;
            }
            if timeout.is_zero() {
                return false;
            }
            thread::sleep(timeout);
            !self.queue.is_empty()
        }

        fn read(&mut self) -> Event {
            loop {
                if let Some(event_item) = self.queue.pop_front() {
                    return event_item;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// End-to-end input submission through the TUI event pipeline.
    ///
    /// # Panics
    /// Panics if `assert!` conditions fail (e.g., the app unexpectedly requests quit).
    ///
    /// # Errors
    /// Propagates errors from `TuiApp::new()` and `TuiApp::tick()` if initialization or a render
    /// cycle fails.
    #[test]
    fn test_end_to_end_input_submission() -> Result<()> {
        let (mut app, _ui) = TuiApp::new()?;

        // hello + Enter
        let events = vec![
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ];

        let events = events.into_iter().map(Event::Key);
        app.set_event_source(Box::new(TestEventSource::with_events(events)));

        // Drive one tick; it should read events, update input, and submit on Enter
        let should_quit = app.tick()?;
        assert!(!should_quit, "App should not request quit on simple input submission");

        let submitted = app.take_pending_input();
        assert_eq!(submitted.as_deref(), Some("hello"));

        Ok(())
    }
}
