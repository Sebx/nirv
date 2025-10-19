use async_trait::async_trait;
use crate::utils::{
    types::{InternalQuery, DataSource, Column, Predicate, OrderBy},
    error::{NirvResult, NirvError},
};

/// Execution plan node types
#[derive(Debug, Clone)]
pub enum PlanNode {
    /// Scan a table/data source
    TableScan {
        source: DataSource,
        projections: Vec<Column>,
        predicates: Vec<Predicate>,
    },
    /// Apply a limit to results
    Limit {
        count: u64,
        input: Box<PlanNode>,
    },
    /// Sort results
    Sort {
        order_by: OrderBy,
        input: Box<PlanNode>,
    },
    /// Project specific columns
    Projection {
        columns: Vec<Column>,
        input: Box<PlanNode>,
    },
}

/// Complete execution plan for a query
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub nodes: Vec<PlanNode>,
    pub estimated_cost: f64,
}

impl ExecutionPlan {
    /// Create a new empty execution plan
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            estimated_cost: 0.0,
        }
    }
    
    /// Add a node to the execution plan
    pub fn add_node(&mut self, node: PlanNode) {
        self.nodes.push(node);
    }
    
    /// Set the estimated cost for the plan
    pub fn set_estimated_cost(&mut self, cost: f64) {
        self.estimated_cost = cost;
    }
    
    /// Get the root node of the plan
    pub fn root_node(&self) -> Option<&PlanNode> {
        self.nodes.last()
    }
    
    /// Check if the plan is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for query planning functionality
#[async_trait]
pub trait QueryPlanner: Send + Sync {
    /// Create an execution plan for the given query
    async fn create_execution_plan(&self, query: &InternalQuery) -> NirvResult<ExecutionPlan>;
    
    /// Estimate the cost of executing a query
    async fn estimate_cost(&self, query: &InternalQuery) -> NirvResult<f64>;
    
    /// Optimize an execution plan
    async fn optimize_plan(&self, plan: ExecutionPlan) -> NirvResult<ExecutionPlan>;
}

/// Default implementation of QueryPlanner
pub struct DefaultQueryPlanner {
    /// Base cost for table scans
    base_scan_cost: f64,
    /// Cost multiplier for predicates
    predicate_cost_multiplier: f64,
    /// Cost for sorting operations
    sort_cost: f64,
    /// Cost for limit operations
    limit_cost: f64,
}

impl DefaultQueryPlanner {
    /// Create a new query planner with default cost parameters
    pub fn new() -> Self {
        Self {
            base_scan_cost: 1.0,
            predicate_cost_multiplier: 0.1,
            sort_cost: 0.5,
            limit_cost: 0.1,
        }
    }
    
    /// Create a query planner with custom cost parameters
    pub fn with_costs(
        base_scan_cost: f64,
        predicate_cost_multiplier: f64,
        sort_cost: f64,
        limit_cost: f64,
    ) -> Self {
        Self {
            base_scan_cost,
            predicate_cost_multiplier,
            sort_cost,
            limit_cost,
        }
    }
    
    /// Validate that a query has the required components
    fn validate_query(&self, query: &InternalQuery) -> NirvResult<()> {
        if query.sources.is_empty() {
            return Err(NirvError::Internal(
                "No data sources found in query".to_string()
            ));
        }
        
        // For MVP, we only support single-source queries
        if query.sources.len() > 1 {
            return Err(NirvError::Internal(
                "Multi-source queries not supported in MVP".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Create a table scan node for a data source
    fn create_table_scan_node(&self, query: &InternalQuery) -> PlanNode {
        let source = query.sources[0].clone();
        let projections = if query.projections.is_empty() {
            // Default to selecting all columns
            vec![Column {
                name: "*".to_string(),
                alias: None,
                source: source.alias.clone(),
            }]
        } else {
            query.projections.clone()
        };
        
        PlanNode::TableScan {
            source,
            projections,
            predicates: query.predicates.clone(),
        }
    }
    
    /// Add limit node if query has a limit clause
    fn add_limit_node(&self, mut plan: ExecutionPlan, query: &InternalQuery) -> ExecutionPlan {
        if let Some(limit) = query.limit {
            if let Some(last_node) = plan.nodes.last() {
                let limit_node = PlanNode::Limit {
                    count: limit,
                    input: Box::new(last_node.clone()),
                };
                plan.add_node(limit_node);
                plan.estimated_cost += self.limit_cost;
            }
        }
        plan
    }
    
    /// Add sort node if query has ordering
    fn add_sort_node(&self, mut plan: ExecutionPlan, query: &InternalQuery) -> ExecutionPlan {
        if let Some(order_by) = &query.ordering {
            if let Some(last_node) = plan.nodes.last() {
                let sort_node = PlanNode::Sort {
                    order_by: order_by.clone(),
                    input: Box::new(last_node.clone()),
                };
                plan.add_node(sort_node);
                plan.estimated_cost += self.sort_cost;
            }
        }
        plan
    }
    
    /// Calculate the estimated cost for a query
    fn calculate_cost(&self, query: &InternalQuery) -> f64 {
        let mut cost = self.base_scan_cost;
        
        // Add cost for predicates
        cost += query.predicates.len() as f64 * self.predicate_cost_multiplier;
        
        // Add cost for sorting
        if query.ordering.is_some() {
            cost += self.sort_cost;
        }
        
        // Add cost for limit
        if query.limit.is_some() {
            cost += self.limit_cost;
        }
        
        cost
    }
}

impl Default for DefaultQueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueryPlanner for DefaultQueryPlanner {
    async fn create_execution_plan(&self, query: &InternalQuery) -> NirvResult<ExecutionPlan> {
        // Validate the query
        self.validate_query(query)?;
        
        let mut plan = ExecutionPlan::new();
        
        // Create the base table scan node
        let table_scan = self.create_table_scan_node(query);
        plan.add_node(table_scan);
        
        // Calculate base cost
        plan.estimated_cost = self.calculate_cost(query);
        
        // Add sort node if needed (before limit)
        plan = self.add_sort_node(plan, query);
        
        // Add limit node if needed (after sort)
        plan = self.add_limit_node(plan, query);
        
        Ok(plan)
    }
    
    async fn estimate_cost(&self, query: &InternalQuery) -> NirvResult<f64> {
        self.validate_query(query)?;
        Ok(self.calculate_cost(query))
    }
    
    async fn optimize_plan(&self, plan: ExecutionPlan) -> NirvResult<ExecutionPlan> {
        // For MVP, we don't implement complex optimizations
        // Just return the plan as-is
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::types::{QueryOperation, PredicateOperator, PredicateValue, OrderColumn, OrderDirection};

    #[test]
    fn test_execution_plan_creation() {
        let mut plan = ExecutionPlan::new();
        
        assert!(plan.is_empty());
        assert_eq!(plan.estimated_cost, 0.0);
        assert!(plan.root_node().is_none());
        
        let node = PlanNode::TableScan {
            source: DataSource {
                object_type: "mock".to_string(),
                identifier: "test".to_string(),
                alias: None,
            },
            projections: vec![],
            predicates: vec![],
        };
        
        plan.add_node(node);
        plan.set_estimated_cost(1.5);
        
        assert!(!plan.is_empty());
        assert_eq!(plan.estimated_cost, 1.5);
        assert!(plan.root_node().is_some());
    }

    #[test]
    fn test_default_query_planner_creation() {
        let planner = DefaultQueryPlanner::new();
        
        assert_eq!(planner.base_scan_cost, 1.0);
        assert_eq!(planner.predicate_cost_multiplier, 0.1);
        assert_eq!(planner.sort_cost, 0.5);
        assert_eq!(planner.limit_cost, 0.1);
    }

    #[test]
    fn test_query_planner_with_custom_costs() {
        let planner = DefaultQueryPlanner::with_costs(2.0, 0.2, 1.0, 0.2);
        
        assert_eq!(planner.base_scan_cost, 2.0);
        assert_eq!(planner.predicate_cost_multiplier, 0.2);
        assert_eq!(planner.sort_cost, 1.0);
        assert_eq!(planner.limit_cost, 0.2);
    }

    #[tokio::test]
    async fn test_query_planner_validate_empty_query() {
        let planner = DefaultQueryPlanner::new();
        let query = InternalQuery::new(QueryOperation::Select);
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Internal(msg) => {
                assert!(msg.contains("No data sources"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_validate_multi_source_query() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "table1".to_string(),
            alias: None,
        });
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "table2".to_string(),
            alias: None,
        });
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Internal(msg) => {
                assert!(msg.contains("Multi-source queries not supported"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_simple_select() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.nodes.len(), 1);
        assert_eq!(plan.estimated_cost, 1.0); // base_scan_cost
        
        match &plan.nodes[0] {
            PlanNode::TableScan { source, projections, predicates } => {
                assert_eq!(source.object_type, "mock");
                assert_eq!(source.identifier, "users");
                assert_eq!(projections.len(), 1);
                assert_eq!(projections[0].name, "*");
                assert!(predicates.is_empty());
            }
            _ => panic!("Expected TableScan node"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_with_projections() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: Some("u".to_string()),
        });
        query.projections.push(Column {
            name: "name".to_string(),
            alias: Some("user_name".to_string()),
            source: Some("u".to_string()),
        });
        query.projections.push(Column {
            name: "email".to_string(),
            alias: None,
            source: Some("u".to_string()),
        });
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        match &plan.nodes[0] {
            PlanNode::TableScan { projections, .. } => {
                assert_eq!(projections.len(), 2);
                assert_eq!(projections[0].name, "name");
                assert_eq!(projections[0].alias, Some("user_name".to_string()));
                assert_eq!(projections[1].name, "email");
                assert_eq!(projections[1].alias, None);
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
        query.predicates.push(Predicate {
            column: "age".to_string(),
            operator: PredicateOperator::GreaterThan,
            value: PredicateValue::Integer(18),
        });
        query.predicates.push(Predicate {
            column: "status".to_string(),
            operator: PredicateOperator::Equal,
            value: PredicateValue::String("active".to_string()),
        });
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.estimated_cost, 1.2); // base_scan_cost + 2 * predicate_cost_multiplier
        
        match &plan.nodes[0] {
            PlanNode::TableScan { predicates, .. } => {
                assert_eq!(predicates.len(), 2);
                assert_eq!(predicates[0].column, "age");
                assert_eq!(predicates[1].column, "status");
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
        query.limit = Some(10);
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.nodes.len(), 2); // TableScan + Limit
        assert_eq!(plan.estimated_cost, 1.1); // base_scan_cost + limit_cost
        
        match &plan.nodes[1] {
            PlanNode::Limit { count, .. } => {
                assert_eq!(*count, 10);
            }
            _ => panic!("Expected Limit node"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_with_ordering() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        query.ordering = Some(OrderBy {
            columns: vec![OrderColumn {
                column: "name".to_string(),
                direction: OrderDirection::Ascending,
            }],
        });
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.nodes.len(), 2); // TableScan + Sort
        assert_eq!(plan.estimated_cost, 1.5); // base_scan_cost + sort_cost
        
        match &plan.nodes[1] {
            PlanNode::Sort { order_by, .. } => {
                assert_eq!(order_by.columns.len(), 1);
                assert_eq!(order_by.columns[0].column, "name");
            }
            _ => panic!("Expected Sort node"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_with_ordering_and_limit() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        query.ordering = Some(OrderBy {
            columns: vec![OrderColumn {
                column: "created_at".to_string(),
                direction: OrderDirection::Descending,
            }],
        });
        query.limit = Some(5);
        
        let result = planner.create_execution_plan(&query).await;
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.nodes.len(), 3); // TableScan + Sort + Limit
        assert_eq!(plan.estimated_cost, 1.6); // base_scan_cost + sort_cost + limit_cost
        
        // Sort should come before Limit
        match &plan.nodes[1] {
            PlanNode::Sort { .. } => {},
            _ => panic!("Expected Sort node at position 1"),
        }
        
        match &plan.nodes[2] {
            PlanNode::Limit { count, .. } => {
                assert_eq!(*count, 5);
            }
            _ => panic!("Expected Limit node at position 2"),
        }
    }

    #[tokio::test]
    async fn test_query_planner_estimate_cost() {
        let planner = DefaultQueryPlanner::new();
        
        let mut query = InternalQuery::new(QueryOperation::Select);
        query.sources.push(DataSource {
            object_type: "mock".to_string(),
            identifier: "users".to_string(),
            alias: None,
        });
        query.predicates.push(Predicate {
            column: "age".to_string(),
            operator: PredicateOperator::GreaterThan,
            value: PredicateValue::Integer(18),
        });
        query.ordering = Some(OrderBy {
            columns: vec![OrderColumn {
                column: "name".to_string(),
                direction: OrderDirection::Ascending,
            }],
        });
        query.limit = Some(10);
        
        let result = planner.estimate_cost(&query).await;
        assert!(result.is_ok());
        
        let cost = result.unwrap();
        assert_eq!(cost, 1.6); // base_scan_cost + predicate_cost + sort_cost + limit_cost
    }

    #[tokio::test]
    async fn test_query_planner_optimize_plan() {
        let planner = DefaultQueryPlanner::new();
        
        let plan = ExecutionPlan {
            nodes: vec![
                PlanNode::TableScan {
                    source: DataSource {
                        object_type: "mock".to_string(),
                        identifier: "users".to_string(),
                        alias: None,
                    },
                    projections: vec![],
                    predicates: vec![],
                }
            ],
            estimated_cost: 1.0,
        };
        
        let result = planner.optimize_plan(plan.clone()).await;
        assert!(result.is_ok());
        
        let optimized_plan = result.unwrap();
        assert_eq!(optimized_plan.nodes.len(), plan.nodes.len());
        assert_eq!(optimized_plan.estimated_cost, plan.estimated_cost);
    }
}