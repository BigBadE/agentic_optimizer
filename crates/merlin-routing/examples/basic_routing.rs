use merlin_routing::{RoutingConfig, RoutingOrchestrator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Agentic Routing Example ===\n");
    
    // Create configuration
    let config = RoutingConfig::default();
    
    println!("Configuration:");
    println!("  Local enabled: {}", config.tiers.local_enabled);
    println!("  Groq enabled: {}", config.tiers.groq_enabled);
    println!("  Max concurrent tasks: {}", config.execution.max_concurrent_tasks);
    println!("  Validation enabled: {}\n", config.validation.enabled);
    
    // Create orchestrator
    let orchestrator = RoutingOrchestrator::new(config);
    
    // Example 1: Simple request
    println!("Example 1: Simple request");
    let request = "Add a comment to the main function";
    println!("Request: {}", request);
    
    let analysis = orchestrator.analyze_request(request).await?;
    println!("Tasks generated: {}", analysis.tasks.len());
    for (i, task) in analysis.tasks.iter().enumerate() {
        println!("  {}. {} (complexity: {:?})", i + 1, task.description, task.complexity);
    }
    println!();
    
    // Example 2: Complex refactor
    println!("Example 2: Complex refactor");
    let request = "Refactor the parser module to use async patterns";
    println!("Request: {}", request);
    
    let analysis = orchestrator.analyze_request(request).await?;
    println!("Tasks generated: {}", analysis.tasks.len());
    for (i, task) in analysis.tasks.iter().enumerate() {
        println!("  {}. {} (complexity: {:?})", i + 1, task.description, task.complexity);
        if !task.dependencies.is_empty() {
            println!("     Dependencies: {} task(s)", task.dependencies.len());
        }
    }
    println!();
    
    // Example 3: Multi-file modification
    println!("Example 3: Multi-file modification");
    let request = "Modify test.rs and main.rs to add error handling";
    println!("Request: {}", request);
    
    let analysis = orchestrator.analyze_request(request).await?;
    println!("Tasks generated: {}", analysis.tasks.len());
    println!("Execution strategy: {:?}", analysis.execution_strategy);
    for task in &analysis.tasks {
        println!("  - {} (files: {})", 
            task.description, 
            task.context_needs.required_files.len()
        );
    }
    println!();
    
    // Example 4: Process complete request (mock execution)
    println!("Example 4: Complete workflow");
    let request = "Add a simple test function";
    println!("Request: {}", request);
    
    let results = orchestrator.process_request(request).await?;
    println!("Execution completed:");
    for result in &results {
        println!("  Task: {} - Success: {} ({}ms)", 
            result.task_id, 
            result.success, 
            result.duration_ms
        );
    }
    
    println!("\n=== Example Complete ===");
    Ok(())
}

