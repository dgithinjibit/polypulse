use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BetTemplate {
    pub id: i64,
    pub category: String,
    pub name: String,
    pub question_template: String,
    pub variables: serde_json::Value,
    pub suggested_end_time: Option<String>,
    pub usage_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: String,
    pub required: bool,
    pub autocomplete: bool,
}

pub struct BetTemplateService {
    db: PgPool,
}

impl BetTemplateService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
    
    /// Get templates by category
    pub async fn get_by_category(&self, category: &str) -> Result<Vec<BetTemplate>, AppError> {
        let templates = sqlx::query_as!(
            BetTemplate,
            r#"
            SELECT id, category, name, question_template, variables, 
                   suggested_end_time, usage_count
            FROM bet_templates
            WHERE category = $1
            ORDER BY usage_count DESC
            LIMIT 20
            "#,
            category
        )
        .fetch_all(&self.db)
        .await?;
        
        Ok(templates)
    }
    
    /// Get all templates
    pub async fn get_all(&self) -> Result<Vec<BetTemplate>, AppError> {
        let templates = sqlx::query_as!(
            BetTemplate,
            r#"
            SELECT id, category, name, question_template, variables,
                   suggested_end_time, usage_count
            FROM bet_templates
            ORDER BY category, usage_count DESC
            "#
        )
        .fetch_all(&self.db)
        .await?;
        
        Ok(templates)
    }
    
    /// Fill template with variables
    pub fn fill_template(
        &self,
        template: &BetTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<String, AppError> {
        let mut question = template.question_template.clone();
        
        // Replace placeholders
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            question = question.replace(&placeholder, value);
        }
        
        // Validate all placeholders filled
        if question.contains("{{") {
            return Err(AppError::BadRequest(
                "Missing required template variables".to_string(),
            ));
        }
        
        Ok(question)
    }
    
    /// Increment usage count
    pub async fn increment_usage(&self, template_id: i64) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            UPDATE bet_templates
            SET usage_count = usage_count + 1
            WHERE id = $1
            "#,
            template_id
        )
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Create new template
    pub async fn create_template(
        &self,
        category: String,
        name: String,
        question_template: String,
        variables: Vec<TemplateVariable>,
        suggested_end_time: Option<String>,
    ) -> Result<BetTemplate, AppError> {
        let variables_json = serde_json::to_value(variables)?;
        
        let template = sqlx::query_as!(
            BetTemplate,
            r#"
            INSERT INTO bet_templates 
            (category, name, question_template, variables, suggested_end_time, usage_count)
            VALUES ($1, $2, $3, $4, $5, 0)
            RETURNING id, category, name, question_template, variables, suggested_end_time, usage_count
            "#,
            category,
            name,
            question_template,
            variables_json,
            suggested_end_time
        )
        .fetch_one(&self.db)
        .await?;
        
        Ok(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Feature: polypulse-enhancements, Property 6: Template Variable Substitution
    #[test]
    fn test_fill_template_simple() {
        let service = BetTemplateService {
            db: PgPool::connect_lazy("").unwrap(),
        };
        
        let template = BetTemplate {
            id: 1,
            category: "crypto".to_string(),
            name: "Price Prediction".to_string(),
            question_template: "Will {{crypto}} reach ${{price}} by {{date}}?".to_string(),
            variables: serde_json::json!([]),
            suggested_end_time: None,
            usage_count: 0,
        };
        
        let mut vars = HashMap::new();
        vars.insert("crypto".to_string(), "BTC".to_string());
        vars.insert("price".to_string(), "50000".to_string());
        vars.insert("date".to_string(), "Dec 31".to_string());
        
        let result = service.fill_template(&template, &vars).unwrap();
        
        assert_eq!(result, "Will BTC reach $50000 by Dec 31?");
        assert!(!result.contains("{{"));
        assert!(!result.contains("}}"));
    }
    
    #[test]
    fn test_fill_template_missing_variable() {
        let service = BetTemplateService {
            db: PgPool::connect_lazy("").unwrap(),
        };
        
        let template = BetTemplate {
            id: 1,
            category: "crypto".to_string(),
            name: "Price Prediction".to_string(),
            question_template: "Will {{crypto}} reach ${{price}}?".to_string(),
            variables: serde_json::json!([]),
            suggested_end_time: None,
            usage_count: 0,
        };
        
        let mut vars = HashMap::new();
        vars.insert("crypto".to_string(), "BTC".to_string());
        // Missing "price" variable
        
        let result = service.fill_template(&template, &vars);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_fill_template_multiple_same_variable() {
        let service = BetTemplateService {
            db: PgPool::connect_lazy("").unwrap(),
        };
        
        let template = BetTemplate {
            id: 1,
            category: "sports".to_string(),
            name: "Team Match".to_string(),
            question_template: "Will {{team}} beat {{team}} in the finals?".to_string(),
            variables: serde_json::json!([]),
            suggested_end_time: None,
            usage_count: 0,
        };
        
        let mut vars = HashMap::new();
        vars.insert("team".to_string(), "Lakers".to_string());
        
        let result = service.fill_template(&template, &vars).unwrap();
        
        // Both instances should be replaced
        assert_eq!(result, "Will Lakers beat Lakers in the finals?");
        assert!(!result.contains("{{"));
    }
    
    #[test]
    fn test_fill_template_no_variables() {
        let service = BetTemplateService {
            db: PgPool::connect_lazy("").unwrap(),
        };
        
        let template = BetTemplate {
            id: 1,
            category: "general".to_string(),
            name: "Simple Question".to_string(),
            question_template: "Will it rain tomorrow?".to_string(),
            variables: serde_json::json!([]),
            suggested_end_time: None,
            usage_count: 0,
        };
        
        let vars = HashMap::new();
        
        let result = service.fill_template(&template, &vars).unwrap();
        
        assert_eq!(result, "Will it rain tomorrow?");
    }
    
    #[test]
    fn test_fill_template_special_characters() {
        let service = BetTemplateService {
            db: PgPool::connect_lazy("").unwrap(),
        };
        
        let template = BetTemplate {
            id: 1,
            category: "crypto".to_string(),
            name: "Price with symbols".to_string(),
            question_template: "Will {{crypto}} be > ${{price}}?".to_string(),
            variables: serde_json::json!([]),
            suggested_end_time: None,
            usage_count: 0,
        };
        
        let mut vars = HashMap::new();
        vars.insert("crypto".to_string(), "ETH".to_string());
        vars.insert("price".to_string(), "3,000".to_string());
        
        let result = service.fill_template(&template, &vars).unwrap();
        
        assert_eq!(result, "Will ETH be > $3,000?");
    }
}
