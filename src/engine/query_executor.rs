use async_trait::async_trait;
use std::time::{Duration, Instant};
use crate::{
    engine::{ExecutionPlan, PlanNode},
    connectors::ConnectorRegistry,
    utils::{
        types::{QueryResult, Row, Value, ColumnMetadata, DataType, InternalQuery, QueryOperation, ConnectorQuery},
        error::{NirvResult, NirvError},
    },
};

/// Trait for query execution functionality
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// Execute an execution plan and return results
    async fn execute_plan(&self, plan: &ExecutionPlan) -> NirvResult<QueryResult>;
    
    /// Execute a single plan node
    async fn execute_node(&self, node: &PlanNode) -> NirvResult<QueryResult>;
    
    /// Set the connector registry for accessing data sources
    fn set_connector_registry(&mut self, registry: ConnectorRegistry);
}

/// Default implementation of QueryExecutor
pub struct DefaultQueryExecutor {
    /// Registry of available connectors
    connector_registry: Option<ConnectorRegistry>,
}

impl DefaultQueryExecutor {
    /// Create a new query executor
    pub fn new() -> Self {
        Self {
            connector_registry: None,
        }
    }
    
    /// Create a query executor with a connector registry
    pub fn with_connector_registry(registry: ConnectorRegistry) -> Self {
        Self {
            connector_registry: Some(registry),
        }
    }
    
    /// Get a reference to the connector registry
    fn get_connector_registry(&self) -> NirvResult<&ConnectorRegistry> {
        self.connector_registry.as_ref().ok_or_else(|| {
            NirvError::Internal("No connector registry configured".to_string())
        })
    }
    
    /// Execute a table scan operation
    async fn execute_table_scan(
        &self,
        source: &crate::utils::types::DataSource,
        projections: &[crate::utils::types::Column],
        predicates: &[crate::utils::types::Predicate],
    ) -> NirvResult<QueryResult> {
        let registry = self.get_connector_registry()?;
        
        // Try different naming patterns to find the connector
        let possible_names = vec![
            source.object_type.clone(),
            format!("{}_{}", source.object_type, 0),
            format!("{}_connector", source.object_type),
        ];
        
        let mut connector = None;
        for name in &possible_names {
            if let Some(c) = registry.get(name) {
                connector = Some(c);
                break;
            }
        }
        
        let connector = connector.ok_or_else(|| {
            NirvError::Internal(format!("No connector found for type: {}", source.object_type))
        })?;
        
        // Create a connector query
        let mut internal_query = InternalQuery::new(QueryOperation::Select);
        internal_query.sources.push(source.clone());
        internal_query.projections = projections.to_vec();
        internal_query.predicates = predicates.to_vec();
        
        let connector_query = ConnectorQuery {
            connector_type: connector.get_connector_type(),
            query: internal_query,
            connection_params: std::collections::HashMap::new(),
        };
        
        // Execute the query through the connector
        connector.execute_query(connector_query).await
    }
    
    /// Apply a limit to query results
    fn apply_limit(&self, mut result: QueryResult, count: u64) -> QueryResult {
        let limit = count as usize;
        if result.rows.len() > limit {
            result.rows.truncate(limit);
        }
        result
    }
    
    /// Apply sorting to query results
    fn apply_sort(&self, mut result: QueryResult, order_by: &crate::utils::types::OrderBy) -> NirvResult<QueryResult> {
        if order_by.columns.is_empty() {
            return Ok(result);
        }
        
        // For MVP, we'll implement simple single-column sorting
        let sort_column = &order_by.columns[0];
        
        // Find the column index
        let column_index = result.columns.iter()
            .position(|col| col.name == sort_column.column)
            .ok_or_else(|| {
                NirvError::Internal(format!("Sort column '{}' not found in result", sort_column.column))
            })?;
        
        // Sort the rows based on the column value
        result.rows.sort_by(|a, b| {
            let val_a = a.get(column_index).unwrap_or(&Value::Null);
            let val_b = b.get(column_index).unwrap_or(&Value::Null);
            
            let comparison = self.compare_values(val_a, val_b);
            
            match sort_column.direction {
                crate::utils::types::OrderDirection::Ascending => comparison,
                crate::utils::types::OrderDirection::Descending => comparison.reverse(),
            }
        });
        
        Ok(result)
    }
    
    /// Compare two values for sorting
    fn compare_values(&self, a: &Value, b: &Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        
        match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
            (Value::Date(a), Value::Date(b)) => a.cmp(b),
            (Value::DateTime(a), Value::DateTime(b)) => a.cmp(b),
            // For mixed types, convert to string and compare
            _ => format!("{:?}", a).cmp(&format!("{:?}", b)),
        }
    }
    
    /// Apply projection to query results
    fn apply_projection(&self, result: QueryResult, columns: &[crate::utils::types::Column]) -> NirvResult<QueryResult> {
        if columns.is_empty() {
            return Ok(result);
        }
        
        // For MVP, we'll assume projections are already handled in the table scan
        // This is a placeholder for future enhancement
        Ok(result)
    }
    
    /// Aggregate results from multiple operations
    fn aggregate_results(&self, results: Vec<QueryResult>) -> NirvResult<QueryResult> {
        if results.is_empty() {
            return Ok(QueryResult::new());
        }
        
        if results.len() == 1 {
            return Ok(results.into_iter().next().unwrap());
        }
        
        // For MVP, we don't support complex aggregation
        // Just return the first result
        Ok(results.into_iter().next().unwrap())
    }
    
    /// Format the final query result
    fn format_result(&self, mut result: QueryResult, execution_time: Duration) -> QueryResult {
        result.execution_time = execution_time;
        
        // Ensure we have proper column metadata if missing
        if result.columns.is_empty() && !result.rows.is_empty() {
            let first_row = &result.rows[0];
            for (i, value) in first_row.values.iter().enumerate() {
                let data_type = match value {
                    Value::Integer(_) => DataType::Integer,
                    Value::Float(_) => DataType::Float,
                    Value::Text(_) => DataType::Text,
                    Value::Boolean(_) => DataType::Boolean,
                    Value::Date(_) => DataType::Date,
                    Value::DateTime(_) => DataType::DateTime,
                    Value::Json(_) => DataType::Json,
                    Value::Binary(_) => DataType::Binary,
                    Value::Null => DataType::Text, // Default for null values
                };
                
                result.columns.push(ColumnMetadata {
                    name: format!("column_{}", i),
                    data_type,
                    nullable: true,
                });
            }
        }
        
        result
    }
}

impl Default for DefaultQueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueryExecutor for DefaultQueryExecutor {
    async fn execute_plan(&self, plan: &ExecutionPlan) -> NirvResult<QueryResult> {
        let start_time = Instant::now();
        
        if plan.is_empty() {
            let execution_time = start_time.elapsed();
            return Ok(self.format_result(QueryResult::new(), execution_time));
        }
        
        // Execute the root node (last node in the plan)
        // The root node will recursively execute its dependencies
        let root_node = plan.root_node().ok_or_else(|| {
            NirvError::Internal("No root node found in execution plan".to_string())
        })?;
        
        let final_result = self.execute_node(root_node).await?;
        
        let execution_time = start_time.elapsed();
        Ok(self.format_result(final_result, execution_time))
    }
    
    async fn execute_node(&self, node: &PlanNode) -> NirvResult<QueryResult> {
        match node {
            PlanNode::TableScan { source, projections, predicates } => {
                self.execute_table_scan(source, projections, predicates).await
            }
            PlanNode::Limit { count, input } => {
                let input_result = self.execute_node(input).await?;
                Ok(self.apply_limit(input_result, *count))
            }
            PlanNode::Sort { order_by, input } => {
                let input_result = self.execute_node(input).await?;
                self.apply_sort(input_result, order_by)
            }
            PlanNode::Projection { columns, input } => {
                let input_result = self.execute_node(input).await?;
                self.apply_projection(input_result, columns)
            }
        }
    }
    
    fn set_connector_registry(&mut self, registry: ConnectorRegistry) {
        self.connector_registry = Some(registry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::{ExecutionPlan, PlanNode},
        connectors::{MockConnector, ConnectorRegistry},
        utils::types::{DataSource, Column, Predicate, PredicateOperator, PredicateValue, OrderBy, OrderColumn, OrderDirection},
    };

    #[test]
    fn test_default_query_executor_creation() {
        let executor = DefaultQueryExecutor::new();
        
        // Should not have a connector registry initially
        assert!(executor.get_connector_registry().is_err());
    }

    #[test]
    fn test_query_executor_with_connector_registry() {
        let registry = ConnectorRegistry::new();
        let executor = DefaultQueryExecutor::with_connector_registry(registry);
        
        // Should have a connector registry
        assert!(executor.get_connector_registry().is_ok());
    }

    #[test]
    fn test_query_executor_set_connector_registry() {
        let mut executor = DefaultQueryExecutor::new();
        let registry = ConnectorRegistry::new();
        
        executor.set_connector_registry(registry);
        
        // Should now have a connector registry
        assert!(executor.get_connector_registry().is_ok());
    }

    #[tokio::test]
    async fn test_query_executor_empty_plan() {
        let executor = DefaultQueryExecutor::new();
        let plan = ExecutionPlan::new();
        
        let result = executor.execute_plan(&plan).await;
        assert!(result.is_ok());
        
        let query_result = result.unwrap();
        assert!(query_result.is_empty());
        assert!(query_result.execution_time > Duration::from_millis(0));
    }

    #[tokio::test]
    async fn test_query_executor_no_connector_registry() {
        let executor = DefaultQueryExecutor::new();
        
        let plan = ExecutionPlan {
            nodes: vec![
                PlanNode::TableScan {
                    source: DataSource {
                        object_type: "mock".to_string(),
                        identifier: "test".to_string(),
                        alias: None,
                    },
                    projections: vec![],
                    predicates: vec![],
                }
            ],
            estimated_cost: 1.0,
        };
        
        let result = executor.execute_plan(&plan).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Internal(msg) => {
                assert!(msg.contains("No connector registry"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_apply_limit() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.rows = vec![
            Row::new(vec![Value::Integer(1)]),
            Row::new(vec![Value::Integer(2)]),
            Row::new(vec![Value::Integer(3)]),
            Row::new(vec![Value::Integer(4)]),
            Row::new(vec![Value::Integer(5)]),
        ];
        
        let limited_result = executor.apply_limit(result, 3);
        assert_eq!(limited_result.row_count(), 3);
        
        // Check that the first 3 rows are preserved
        assert_eq!(limited_result.rows[0].get(0), Some(&Value::Integer(1)));
        assert_eq!(limited_result.rows[1].get(0), Some(&Value::Integer(2)));
        assert_eq!(limited_result.rows[2].get(0), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_apply_limit_no_truncation() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.rows = vec![
            Row::new(vec![Value::Integer(1)]),
            Row::new(vec![Value::Integer(2)]),
        ];
        
        let limited_result = executor.apply_limit(result, 5);
        assert_eq!(limited_result.row_count(), 2); // No truncation needed
    }

    #[test]
    fn test_compare_values() {
        let executor = DefaultQueryExecutor::new();
        
        // Test integer comparison
        assert_eq!(
            executor.compare_values(&Value::Integer(1), &Value::Integer(2)),
            std::cmp::Ordering::Less
        );
        
        // Test string comparison
        assert_eq!(
            executor.compare_values(&Value::Text("apple".to_string()), &Value::Text("banana".to_string())),
            std::cmp::Ordering::Less
        );
        
        // Test null comparison
        assert_eq!(
            executor.compare_values(&Value::Null, &Value::Integer(1)),
            std::cmp::Ordering::Less
        );
        
        // Test equal values
        assert_eq!(
            executor.compare_values(&Value::Integer(5), &Value::Integer(5)),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_apply_sort_ascending() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.columns = vec![
            ColumnMetadata {
                name: "value".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            }
        ];
        result.rows = vec![
            Row::new(vec![Value::Integer(3)]),
            Row::new(vec![Value::Integer(1)]),
            Row::new(vec![Value::Integer(2)]),
        ];
        
        let order_by = OrderBy {
            columns: vec![OrderColumn {
                column: "value".to_string(),
                direction: OrderDirection::Ascending,
            }],
        };
        
        let sorted_result = executor.apply_sort(result, &order_by).unwrap();
        
        assert_eq!(sorted_result.rows[0].get(0), Some(&Value::Integer(1)));
        assert_eq!(sorted_result.rows[1].get(0), Some(&Value::Integer(2)));
        assert_eq!(sorted_result.rows[2].get(0), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_apply_sort_descending() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.columns = vec![
            ColumnMetadata {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
            }
        ];
        result.rows = vec![
            Row::new(vec![Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Text("Charlie".to_string())]),
            Row::new(vec![Value::Text("Bob".to_string())]),
        ];
        
        let order_by = OrderBy {
            columns: vec![OrderColumn {
                column: "name".to_string(),
                direction: OrderDirection::Descending,
            }],
        };
        
        let sorted_result = executor.apply_sort(result, &order_by).unwrap();
        
        assert_eq!(sorted_result.rows[0].get(0), Some(&Value::Text("Charlie".to_string())));
        assert_eq!(sorted_result.rows[1].get(0), Some(&Value::Text("Bob".to_string())));
        assert_eq!(sorted_result.rows[2].get(0), Some(&Value::Text("Alice".to_string())));
    }

    #[test]
    fn test_apply_sort_nonexistent_column() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.columns = vec![
            ColumnMetadata {
                name: "value".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            }
        ];
        result.rows = vec![Row::new(vec![Value::Integer(1)])];
        
        let order_by = OrderBy {
            columns: vec![OrderColumn {
                column: "nonexistent".to_string(),
                direction: OrderDirection::Ascending,
            }],
        };
        
        let result = executor.apply_sort(result, &order_by);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            NirvError::Internal(msg) => {
                assert!(msg.contains("Sort column 'nonexistent' not found"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_format_result() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.rows = vec![
            Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
            Row::new(vec![Value::Integer(2), Value::Text("Bob".to_string())]),
        ];
        
        let execution_time = Duration::from_millis(100);
        let formatted_result = executor.format_result(result, execution_time);
        
        assert_eq!(formatted_result.execution_time, execution_time);
        assert_eq!(formatted_result.columns.len(), 2);
        assert_eq!(formatted_result.columns[0].name, "column_0");
        assert_eq!(formatted_result.columns[0].data_type, DataType::Integer);
        assert_eq!(formatted_result.columns[1].name, "column_1");
        assert_eq!(formatted_result.columns[1].data_type, DataType::Text);
    }

    #[test]
    fn test_format_result_with_existing_columns() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result = QueryResult::new();
        result.columns = vec![
            ColumnMetadata {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            }
        ];
        result.rows = vec![Row::new(vec![Value::Integer(1)])];
        
        let execution_time = Duration::from_millis(50);
        let formatted_result = executor.format_result(result, execution_time);
        
        assert_eq!(formatted_result.execution_time, execution_time);
        assert_eq!(formatted_result.columns.len(), 1);
        assert_eq!(formatted_result.columns[0].name, "id");
    }

    #[test]
    fn test_aggregate_results_empty() {
        let executor = DefaultQueryExecutor::new();
        
        let result = executor.aggregate_results(vec![]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_results_single() {
        let executor = DefaultQueryExecutor::new();
        
        let mut query_result = QueryResult::new();
        query_result.rows = vec![Row::new(vec![Value::Integer(1)])];
        
        let result = executor.aggregate_results(vec![query_result]).unwrap();
        assert_eq!(result.row_count(), 1);
    }

    #[test]
    fn test_aggregate_results_multiple() {
        let executor = DefaultQueryExecutor::new();
        
        let mut result1 = QueryResult::new();
        result1.rows = vec![Row::new(vec![Value::Integer(1)])];
        
        let mut result2 = QueryResult::new();
        result2.rows = vec![Row::new(vec![Value::Integer(2)])];
        
        // For MVP, should return the first result
        let result = executor.aggregate_results(vec![result1, result2]).unwrap();
        assert_eq!(result.row_count(), 1);
        assert_eq!(result.rows[0].get(0), Some(&Value::Integer(1)));
    }
}