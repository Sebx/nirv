use nirv_engine::{
    engine::{QueryPlanner, ExecutionPlan, PlanNode, DefaultQueryPlanner},
    utils::{
        types::{InternalQuery, QueryOperation, DataSource, Column, Predicate, PredicateOperator, PredicateValue},
        error::{NirvResult, NirvError},
    },
};

#[tokio::test]
async fn test_query_planner_single_source_select() {
    let planner = DefaultQueryPlanner::new();
    
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "mock".to_string(),
        identifier: "users".to_string(),
        alias: Some("u".to_string()),
    });
    query.projections.push(Column {
        name: "*".to_string(),
        alias: None,
        source: Some("u".to_string()),
    });
    
    let result = planner.create_execution_plan(&query).await;
    assert!(result.is_ok());
    
    let plan = result.unwrap();
    assert_eq!(plan.nodes.len(), 1);
    
    match &plan.nodes[0] {
        PlanNode::TableScan { source, projections, .. } => {
            assert_eq!(source.object_type, "mock");
            assert_eq!(source.identifier, "users");
            assert_eq!(projections.len(), 1);
        }
        _ => panic!("Expected TableScan node"),
    }
}

#[tokio::test]
async fn test_query_planner_with_predicates() {
    let planner = DefaultQueryPlanner::new();
    
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "mock".to_string(),
        identifier: "users".to_string(),
        alias: None,
    });
    query.projections.push(Column {
        name: "name".to_string(),
        alias: None,
        source: None,
    });
    query.predicates.push(Predicate {
        column: "age".to_string(),
        operator: PredicateOperator::GreaterThan,
        value: PredicateValue::Integer(18),
    });
    
    let result = planner.create_execution_plan(&query).await;
    assert!(result.is_ok());
    
    let plan = result.unwrap();
    assert_eq!(plan.nodes.len(), 1);
    
    match &plan.nodes[0] {
        PlanNode::TableScan { predicates, .. } => {
            assert_eq!(predicates.len(), 1);
            assert_eq!(predicates[0].column, "age");
        }
        _ => panic!("Expected TableScan node"),
    }
}

#[tokio::test]
async fn test_query_planner_with_limit() {
    let planner = DefaultQueryPlanner::new();
    
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "mock".to_string(),
        identifier: "users".to_string(),
        alias: None,
    });
    query.projections.push(Column {
        name: "*".to_string(),
        alias: None,
        source: None,
    });
    query.limit = Some(10);
    
    let result = planner.create_execution_plan(&query).await;
    assert!(result.is_ok());
    
    let plan = result.unwrap();
    assert_eq!(plan.nodes.len(), 2); // TableScan + Limit
    
    match &plan.nodes[1] {
        PlanNode::Limit { count, .. } => {
            assert_eq!(*count, 10);
        }
        _ => panic!("Expected Limit node"),
    }
}

#[tokio::test]
async fn test_query_planner_empty_query() {
    let planner = DefaultQueryPlanner::new();
    let query = InternalQuery::new(QueryOperation::Select);
    
    let result = planner.create_execution_plan(&query).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        NirvError::Internal(msg) => {
            assert!(msg.contains("No data sources"));
        }
        _ => panic!("Expected Internal error for empty query"),
    }
}

#[tokio::test]
async fn test_query_planner_cost_estimation() {
    let planner = DefaultQueryPlanner::new();
    
    let mut query = InternalQuery::new(QueryOperation::Select);
    query.sources.push(DataSource {
        object_type: "mock".to_string(),
        identifier: "large_table".to_string(),
        alias: None,
    });
    query.projections.push(Column {
        name: "*".to_string(),
        alias: None,
        source: None,
    });
    
    let result = planner.create_execution_plan(&query).await;
    assert!(result.is_ok());
    
    let plan = result.unwrap();
    assert!(plan.estimated_cost > 0.0);
}