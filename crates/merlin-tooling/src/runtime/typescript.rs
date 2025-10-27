//! TypeScript type stripping and code wrapping utilities.

/// Strip TypeScript type annotations to convert to valid JavaScript using SWC
pub fn strip_typescript_types(code: &str) -> String {
    use merlin_deps::swc_common::{FileName, GLOBALS, Globals, Mark, SourceMap, sync::Lrc};
    use merlin_deps::swc_ecma_ast::EsVersion;
    use merlin_deps::swc_ecma_codegen::{Config as CodegenConfig, Emitter, text_writer::JsWriter};
    use merlin_deps::swc_ecma_parser::{Syntax, TsSyntax, parse_file_as_program};
    use merlin_deps::swc_ecma_transforms_typescript::strip;

    // Create a source map
    let source_map = Lrc::new(SourceMap::default());
    let source_file = source_map.new_source_file(Lrc::new(FileName::Anon), code.to_owned());

    // Configure TypeScript parser
    let syntax = Syntax::Typescript(TsSyntax {
        tsx: false,
        decorators: false,
        dts: false,
        no_early_errors: true,
        disallow_ambiguous_jsx_like: false,
    });

    // Parse the TypeScript code
    let Ok(program) =
        parse_file_as_program(&source_file, syntax, EsVersion::Es2022, None, &mut vec![])
    else {
        // If parsing fails, return original code
        merlin_deps::tracing::warn!("Failed to parse TypeScript code, returning original");
        return code.to_owned();
    };

    // Apply TypeScript stripping transform
    let program = GLOBALS.set(&Globals::default(), || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        // Apply the strip transform
        let mut pass = strip(unresolved_mark, top_level_mark);
        program.apply(&mut pass)
    });

    // Generate JavaScript code
    let mut buf = vec![];
    {
        let writer = JsWriter::new(Lrc::clone(&source_map), "\n", &mut buf, None);
        let mut emitter = Emitter {
            cfg: CodegenConfig::default(),
            cm: Lrc::clone(&source_map),
            comments: None,
            wr: writer,
        };

        if emitter.emit_program(&program).is_err() {
            merlin_deps::tracing::warn!("Failed to emit JavaScript code, returning original");
            return code.to_owned();
        }
    }

    String::from_utf8(buf).unwrap_or_else(|_| {
        merlin_deps::tracing::warn!(
            "Failed to convert generated code to UTF-8, returning original"
        );
        code.to_owned()
    })
}

/// Wrap code in `agent_code` function if needed
pub fn wrap_code(code: &str) -> String {
    // First strip TypeScript type annotations
    let code_without_types = strip_typescript_types(code);
    let trimmed = code_without_types.trim();

    // Check if code already defines agent_code function (async or sync)
    if trimmed.contains("async function agent_code") {
        // Wrap async function call in async IIFE for compatibility
        format!("{trimmed}\n(async () => await agent_code())()")
    } else if trimmed.contains("function agent_code") {
        // Just call sync function
        format!("{trimmed}\nagent_code();")
    } else {
        // Check if code contains top-level await
        let has_await = trimmed.contains("await ");

        // Check if code contains a top-level return statement
        let has_return = trimmed
            .lines()
            .any(|line| line.trim_start().starts_with("return "));

        if has_await {
            // Wrap in async IIFE to support top-level await
            format!("(async () => {{ {trimmed} }})()")
        } else if has_return {
            // Wrap in IIFE since it has explicit return
            // This handles cases like: function foo() { ... } return foo()
            format!("(function() {{ {trimmed} }})()")
        } else {
            // Evaluate directly for simple expressions
            // This allows statements like "const x = 42; x * 2" to work
            trimmed.to_owned()
        }
    }
}
