Audit Complete - Integration-Tests Crate Analysis

I've completed a comprehensive review of the integration-tests crate. Here are my findings:

✅ Strengths

1. Clean architecture - Tests run actual CLI with fixture-based event injection (no duplicate behavior)
2. Good separation - Each verifier handles one concern (execution, files, UI)
3. Proper tracking - ExecutionResultTracker maintains full history of results
4. Pattern-based mocking - PatternMockProvider cleanly separates test responses from implementation
5. No dead code - All modules are actively used

⚠️ Issues & Recommendations

Critical Issues

1. event_source.rs:71 - Clippy violation in production code
   KeyCode::Char(single.chars().next().unwrap_or(' '))
   - Uses .unwrap_or() which should be fine, but there's an actual .unwrap() used on line 71
   - Risk: Could panic if unexpected key names provided in fixtures
   - Fix: Replace with proper error handling or validation
2. runner.rs:154-179 - Potential race condition
   async fn process_submit_and_await(&mut self) -> Result<TaskCompletionResult> {
   let result = timeout(TokioDuration::from_secs(10), async {
   loop {
   self.tui_app.tick()?;
   if let Some(result) = self.process_completion_events(&mut outputs) {
   return result;
   }
   tokio_time::sleep(TokioDuration::from_millis(10)).await;
   }
   })
   - Issue: Fixed 10ms polling interval is arbitrary and could cause flaky tests
   - Risk: Fast operations might complete between ticks, slow operations might timeout unnecessarily
   - Fix: Consider event-driven approach or make timeout/interval configurable per fixture
3. runner.rs:189-224 - False positive potential
   fn process_completion_events(&mut self, outputs: &mut Vec<String>) -> Option<Result<...>> {
   while let Ok(event) = self.event_receiver.try_recv() {
   match event {
   UiEvent::TaskFailed { error, task_id } => {
   // Creates success TaskResult for failures!
   let task_result = TaskResult { /* ... */ };
   return Some(Ok((Box::new(task_result), outputs.clone())));
   }
   - Issue: Task failures are converted to successful Ok() results with error text
   - Risk: Tests expecting failures might get false passes if they only check Ok/Err and not content
   - Current state: Fixtures do check error_occurred field, so this works, but it's confusing
   - Recommendation: Consider making this clearer - either keep as Err or document why failures become Ok
4. verifier.rs:82-86 - Unconditional success on boolean checks
   if let Some(expected) = exec_verify.all_tasks_completed && expected {
   self.result.add_success("All tasks completed".to_owned());
   }
   - Issue: Only adds success when true, doesn't verify actual state
   - Risk: False passes - these fields don't actually verify anything
   - Fix: Either implement actual verification or remove these fields from ExecutionVerify

Moderate Issues

5. execution_verifier.rs:29-35 - Missing execution treated as infrastructure failure
   let Some(execution_result) = last_execution else {
   result.add_failure("TypeScript execution results not captured - test infrastructure issue".to_owned());
   return;
   };
   - Issue: Good "built to fail" approach, BUT doesn't distinguish between "no TypeScript provided" vs "TypeScript provided but not captured"
   - Risk: Fixtures without TypeScript will fail with confusing errors
   - Fix: Add flag to ExecutionVerify for "expects_no_execution" or similar
6. ui_verifier.rs - Minimal implementation
   - Only verifies: input text, focused pane, task count
   - Doesn't verify: Most of the fields defined in UiVerify struct (task status, output contains, snapshot, progress, etc.)
   - Risk: Fixtures using these fields will silently pass without verification
   - Status: README admits this ("⚠️ UI verification temporarily stubbed out")
   - Fix: Complete implementation or remove unused fields from UiVerify
7. fixture.rs:332-342 - Awkward static pattern for Wait events
   Self::Wait(_) => {
   static EMPTY: VerifyConfig = VerifyConfig { /* ... */ };
   &EMPTY
   }
   - Issue: Works but awkward; leaks implementation detail
   - Fix: Make VerifyConfig Option<VerifyConfig> in events, or use Default::default() pattern

Minor Issues / Code Smell

8. runner.rs:28-29 - Type alias not widely used
   type TaskCompletionResult = (Box<TaskResult>, Vec<String>);
   - Used only in 2 places
   - Not improving readability significantly
   - Suggestion: Inline or create proper struct if it needs methods
9. fixture.rs:165 - #[serde(deny_unknown_fields)] on ExecutionVerify only
   - Only this one struct has it
   - Risk: Typos in other verification structs will be silently ignored
   - Fix: Add to all verification structs for consistency
10. mock_provider.rs:107-114 - "First unused matching pattern" behavior
    .find(|resp| !resp.used && resp.matches(query_text))
    - Behavior: Patterns are consumed on first use
    - Risk: If fixtures expect same pattern to match multiple times, it won't
    - Current: Seems intentional for single-use patterns
    - Note: Works correctly but could be surprising
11. event_source.rs:96-100 - Dead code
    #[allow(dead_code, reason = "Will be used when event loop is implemented")]
    pub fn has_events(&self) -> bool {
    !self.events.is_empty()
    }
    - Marked for future use but actually never needed (poll() returns this info)
    - Fix: Remove

Design Questions

12. Why are 18/58 fixtures failing?
    - Tests show systematic failures in conversation tracking, file modifications, timeouts
    - Risk: Indicates either broken implementation OR tests expecting wrong behavior
    - Action needed: Investigate fixture failures - are they testing correct behavior?
13. No integration with actual LLM providers
    - All tests use mock provider
    - Gap: No tests verify actual provider integration works
    - Note: May be intentional (unit vs integration tests)
14. No timeout configuration per fixture
    - All fixtures get same 10-second timeout
    - Some fixtures timeout (query_context_extraction.json)
    - Fix: Add timeout_seconds field to SetupConfig

📋 Recommended Actions (Priority Order)

1. Fix clippy violation in event_source.rs:71
2. Investigate failing fixtures - 18 failures suggest systemic issues
3. Complete or remove stubbed UI verification - current state is misleading
4. Fix unconditional success checks in verifier.rs:82-86
5. Remove has_events() dead code from event_source.rs
6. Add deny_unknown_fields to all verification structs
7. Consider fixture-level timeout configuration
8. Document TaskFailed→Ok conversion in runner.rs more clearly
9. Add flag for "no execution expected" in ExecutionVerify
10. Make polling interval configurable or event-driven

● The audit is complete. The crate is generally well-structured with good separation of concerns, but has several issues that could cause false passes, flaky tests, or misleading failures. The most concerning are the 18
failing fixtures and incomplete UI verification.
