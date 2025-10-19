use colored::*;
use serde_json::{json, Value as JsonValue};
use base64::prelude::*;
use crate::utils::types::{QueryResult, Value};
use crate::cli::cli_args::OutputFormat;

/// Formats query results for CLI output
pub struct OutputFormatter;

impl OutputFormatter {
    /// Format query results according to the specified format
    pub fn format_result(result: &QueryResult, format: &OutputFormat) -> String {
        match format {
            OutputFormat::Table => Self::format_table(result),
            OutputFormat::Json => Self::format_json(result),
            OutputFormat::Csv => Self::format_csv(result),
        }
    }
    
    /// Format results as a colored table
    fn format_table(result: &QueryResult) -> String {
        if result.is_empty() {
            return "No results found.".dimmed().to_string();
        }
        
        let mut output = String::new();
        
        // Calculate column widths
        let mut col_widths = vec![0; result.columns.len()];
        
        // Header widths
        for (i, col) in result.columns.iter().enumerate() {
            col_widths[i] = col.name.len();
        }
        
        // Data widths
        for row in &result.rows {
            for (i, value) in row.values.iter().enumerate() {
                if i < col_widths.len() {
                    let value_str = Self::value_to_string(value);
                    col_widths[i] = col_widths[i].max(value_str.len());
                }
            }
        }
        
        // Ensure minimum width
        for width in &mut col_widths {
            *width = (*width).max(8);
        }
        
        // Header
        output.push_str(&Self::format_table_separator(&col_widths, true));
        output.push('|');
        for (i, col) in result.columns.iter().enumerate() {
            output.push_str(&format!(" {:<width$} |", 
                col.name.bold().cyan(), 
                width = col_widths[i]
            ));
        }
        output.push('\n');
        output.push_str(&Self::format_table_separator(&col_widths, false));
        
        // Data rows
        for row in &result.rows {
            output.push('|');
            for (i, value) in row.values.iter().enumerate() {
                if i < col_widths.len() {
                    let formatted_value = Self::format_value_colored(value);
                    output.push_str(&format!(" {:<width$} |", 
                        formatted_value, 
                        width = col_widths[i]
                    ));
                }
            }
            output.push('\n');
        }
        
        output.push_str(&Self::format_table_separator(&col_widths, true));
        
        // Footer with metadata
        output.push_str(&format!("\n{} {} in {:.2}ms\n", 
            result.row_count().to_string().green().bold(),
            if result.row_count() == 1 { "row" } else { "rows" },
            result.execution_time.as_millis()
        ));
        
        output
    }
    
    /// Format table separator line
    fn format_table_separator(col_widths: &[usize], is_border: bool) -> String {
        let mut separator = String::new();
        
        if is_border {
            separator.push('+');
            for &width in col_widths {
                separator.push_str(&"-".repeat(width + 2));
                separator.push('+');
            }
        } else {
            separator.push('|');
            for &width in col_widths {
                separator.push_str(&"-".repeat(width + 2));
                separator.push('|');
            }
        }
        
        separator.push('\n');
        separator
    }
    
    /// Format results as JSON
    fn format_json(result: &QueryResult) -> String {
        let mut rows = Vec::new();
        
        for row in &result.rows {
            let mut row_obj = serde_json::Map::new();
            
            for (i, value) in row.values.iter().enumerate() {
                if let Some(col) = result.columns.get(i) {
                    let json_value = Self::value_to_json(value);
                    row_obj.insert(col.name.clone(), json_value);
                }
            }
            
            rows.push(JsonValue::Object(row_obj));
        }
        
        let output = json!({
            "data": rows,
            "metadata": {
                "columns": result.columns.iter().map(|col| {
                    json!({
                        "name": col.name,
                        "type": format!("{:?}", col.data_type),
                        "nullable": col.nullable
                    })
                }).collect::<Vec<_>>(),
                "row_count": result.row_count(),
                "execution_time_ms": result.execution_time.as_millis()
            }
        });
        
        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Format results as CSV
    fn format_csv(result: &QueryResult) -> String {
        let mut output = String::new();
        
        // Header
        let headers: Vec<String> = result.columns.iter()
            .map(|col| Self::escape_csv_field(&col.name))
            .collect();
        output.push_str(&headers.join(","));
        output.push('\n');
        
        // Data rows
        for row in &result.rows {
            let values: Vec<String> = row.values.iter()
                .map(|value| Self::escape_csv_field(&Self::value_to_string(value)))
                .collect();
            output.push_str(&values.join(","));
            output.push('\n');
        }
        
        output
    }
    
    /// Convert a Value to a display string
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::Text(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => format!("{:.2}", f),
            Value::Boolean(b) => b.to_string(),
            Value::Date(d) => d.clone(),
            Value::DateTime(dt) => dt.clone(),
            Value::Json(j) => j.clone(),
            Value::Binary(b) => format!("<binary: {} bytes>", b.len()),
            Value::Null => "NULL".to_string(),
        }
    }
    
    /// Convert a Value to a colored string for table display
    fn format_value_colored(value: &Value) -> ColoredString {
        match value {
            Value::Text(s) => s.normal(),
            Value::Integer(i) => i.to_string().blue(),
            Value::Float(f) => format!("{:.2}", f).blue(),
            Value::Boolean(true) => "true".green(),
            Value::Boolean(false) => "false".red(),
            Value::Date(d) => d.yellow(),
            Value::DateTime(dt) => dt.yellow(),
            Value::Json(j) => j.magenta(),
            Value::Binary(b) => format!("<binary: {} bytes>", b.len()).cyan(),
            Value::Null => "NULL".dimmed(),
        }
    }
    
    /// Convert a Value to JSON
    fn value_to_json(value: &Value) -> JsonValue {
        match value {
            Value::Text(s) => JsonValue::String(s.clone()),
            Value::Integer(i) => JsonValue::Number((*i).into()),
            Value::Float(f) => {
                if let Some(num) = serde_json::Number::from_f64(*f) {
                    JsonValue::Number(num)
                } else {
                    JsonValue::Null
                }
            },
            Value::Boolean(b) => JsonValue::Bool(*b),
            Value::Date(d) => JsonValue::String(d.clone()),
            Value::DateTime(dt) => JsonValue::String(dt.clone()),
            Value::Json(j) => {
                serde_json::from_str(j).unwrap_or(JsonValue::String(j.clone()))
            },
            Value::Binary(b) => JsonValue::String(BASE64_STANDARD.encode(b)),
            Value::Null => JsonValue::Null,
        }
    }
    
    /// Escape CSV field if it contains special characters
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }
    
    /// Format error message for CLI display
    pub fn format_error(error: &crate::utils::error::NirvError) -> String {
        format!("{} {}", "Error:".red().bold(), error.to_string().red())
    }
    
    /// Format success message for CLI display
    pub fn format_success(message: &str) -> String {
        format!("{} {}", "Success:".green().bold(), message)
    }
    
    /// Format info message for CLI display
    pub fn format_info(message: &str) -> String {
        format!("{} {}", "Info:".blue().bold(), message)
    }
}