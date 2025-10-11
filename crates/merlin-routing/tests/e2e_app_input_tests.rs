//! End-to-end tests that drive the TUI app via an injected input event source.

mod common;

#[cfg(test)]
mod tests {
    use super::common::TestEventSource;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use merlin_routing::Result;
    use merlin_routing::user_interface::TuiApp;

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
        assert!(
            !should_quit,
            "App should not request quit on simple input submission"
        );

        let submitted = app.take_pending_input();
        assert_eq!(submitted.as_deref(), Some("hello"));

        Ok(())
    }
}
