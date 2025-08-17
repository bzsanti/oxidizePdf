//! Tests para aumentar cobertura de LÍNEAS en forms/calculations.rs
//! Enfocado en branches no ejecutados y paths de error

use oxidize_pdf::forms::calculations::{
    CalculationEngine, CalculationError, Expression, FieldValue,
};
use std::collections::HashMap;

#[test]
fn test_field_value_conversions() {
    // Test todas las conversiones de FieldValue
    let num_val = FieldValue::Number(42.5);
    let bool_val = FieldValue::Boolean(true);
    let text_val = FieldValue::Text("123.45".to_string());
    let null_val = FieldValue::Null;

    // Conversión a número
    assert_eq!(num_val.to_number(), Some(42.5));
    assert_eq!(bool_val.to_number(), Some(1.0));
    assert_eq!(text_val.to_number(), Some(123.45));
    assert_eq!(null_val.to_number(), None);

    // Conversión a booleano
    assert_eq!(num_val.to_boolean(), true);
    assert_eq!(bool_val.to_boolean(), true);
    assert_eq!(FieldValue::Number(0.0).to_boolean(), false);
    assert_eq!(FieldValue::Text("".to_string()).to_boolean(), false);
    assert_eq!(null_val.to_boolean(), false);

    // Conversión a texto
    assert_eq!(num_val.to_text(), "42.5");
    assert_eq!(bool_val.to_text(), "true");
    assert_eq!(text_val.to_text(), "123.45");
    assert_eq!(null_val.to_text(), "");
}

#[test]
fn test_expression_evaluation_edge_cases() {
    let mut engine = CalculationEngine::new();

    // Test división por cero
    engine.set_field_value("a", FieldValue::Number(10.0));
    engine.set_field_value("b", FieldValue::Number(0.0));

    let div_by_zero = Expression::BinaryOp {
        op: "/".to_string(),
        left: Box::new(Expression::Field("a".to_string())),
        right: Box::new(Expression::Field("b".to_string())),
    };

    let result = engine.evaluate_expression(&div_by_zero);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(0.0));

    // Test operaciones con null
    engine.set_field_value("null_field", FieldValue::Null);

    let null_op = Expression::BinaryOp {
        op: "+".to_string(),
        left: Box::new(Expression::Field("null_field".to_string())),
        right: Box::new(Expression::Number(10.0)),
    };

    let result = engine.evaluate_expression(&null_op);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(10.0));
}

#[test]
fn test_function_calls() {
    let mut engine = CalculationEngine::new();

    // Test función SUM
    engine.set_field_value("field1", FieldValue::Number(10.0));
    engine.set_field_value("field2", FieldValue::Number(20.0));
    engine.set_field_value("field3", FieldValue::Number(30.0));

    let sum_expr = Expression::FunctionCall {
        name: "SUM".to_string(),
        args: vec![
            Expression::Field("field1".to_string()),
            Expression::Field("field2".to_string()),
            Expression::Field("field3".to_string()),
        ],
    };

    let result = engine.evaluate_expression(&sum_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(60.0));

    // Test función AVG
    let avg_expr = Expression::FunctionCall {
        name: "AVG".to_string(),
        args: vec![
            Expression::Field("field1".to_string()),
            Expression::Field("field2".to_string()),
            Expression::Field("field3".to_string()),
        ],
    };

    let result = engine.evaluate_expression(&avg_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(20.0));

    // Test función MIN
    let min_expr = Expression::FunctionCall {
        name: "MIN".to_string(),
        args: vec![
            Expression::Field("field1".to_string()),
            Expression::Field("field2".to_string()),
            Expression::Field("field3".to_string()),
        ],
    };

    let result = engine.evaluate_expression(&min_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(10.0));

    // Test función MAX
    let max_expr = Expression::FunctionCall {
        name: "MAX".to_string(),
        args: vec![
            Expression::Field("field1".to_string()),
            Expression::Field("field2".to_string()),
            Expression::Field("field3".to_string()),
        ],
    };

    let result = engine.evaluate_expression(&max_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(30.0));

    // Test función COUNT
    let count_expr = Expression::FunctionCall {
        name: "COUNT".to_string(),
        args: vec![
            Expression::Field("field1".to_string()),
            Expression::Field("field2".to_string()),
            Expression::Field("field3".to_string()),
        ],
    };

    let result = engine.evaluate_expression(&count_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(3.0));

    // Test función desconocida
    let unknown_expr = Expression::FunctionCall {
        name: "UNKNOWN_FUNCTION".to_string(),
        args: vec![],
    };

    let result = engine.evaluate_expression(&unknown_expr);
    assert!(result.is_err());
}

#[test]
fn test_conditional_expressions() {
    let mut engine = CalculationEngine::new();

    engine.set_field_value("age", FieldValue::Number(25.0));

    // Test IF con condición verdadera
    let if_true = Expression::Conditional {
        condition: Box::new(Expression::BinaryOp {
            op: ">".to_string(),
            left: Box::new(Expression::Field("age".to_string())),
            right: Box::new(Expression::Number(18.0)),
        }),
        then_expr: Box::new(Expression::Text("Adult".to_string())),
        else_expr: Box::new(Expression::Text("Minor".to_string())),
    };

    let result = engine.evaluate_expression(&if_true);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Text("Adult".to_string()));

    // Test IF con condición falsa
    engine.set_field_value("age", FieldValue::Number(15.0));

    let result = engine.evaluate_expression(&if_true);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Text("Minor".to_string()));
}

#[test]
fn test_logical_operations() {
    let mut engine = CalculationEngine::new();

    engine.set_field_value("a", FieldValue::Boolean(true));
    engine.set_field_value("b", FieldValue::Boolean(false));

    // Test AND
    let and_expr = Expression::BinaryOp {
        op: "&&".to_string(),
        left: Box::new(Expression::Field("a".to_string())),
        right: Box::new(Expression::Field("b".to_string())),
    };

    let result = engine.evaluate_expression(&and_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Boolean(false));

    // Test OR
    let or_expr = Expression::BinaryOp {
        op: "||".to_string(),
        left: Box::new(Expression::Field("a".to_string())),
        right: Box::new(Expression::Field("b".to_string())),
    };

    let result = engine.evaluate_expression(&or_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Boolean(true));

    // Test NOT (unary)
    let not_expr = Expression::UnaryOp {
        op: "!".to_string(),
        operand: Box::new(Expression::Field("a".to_string())),
    };

    let result = engine.evaluate_expression(&not_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Boolean(false));
}

#[test]
fn test_comparison_operations() {
    let mut engine = CalculationEngine::new();

    engine.set_field_value("x", FieldValue::Number(10.0));
    engine.set_field_value("y", FieldValue::Number(20.0));

    // Test <
    let lt_expr = Expression::BinaryOp {
        op: "<".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Field("y".to_string())),
    };
    assert_eq!(
        engine.evaluate_expression(&lt_expr).unwrap(),
        FieldValue::Boolean(true)
    );

    // Test <=
    let le_expr = Expression::BinaryOp {
        op: "<=".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Field("y".to_string())),
    };
    assert_eq!(
        engine.evaluate_expression(&le_expr).unwrap(),
        FieldValue::Boolean(true)
    );

    // Test >
    let gt_expr = Expression::BinaryOp {
        op: ">".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Field("y".to_string())),
    };
    assert_eq!(
        engine.evaluate_expression(&gt_expr).unwrap(),
        FieldValue::Boolean(false)
    );

    // Test >=
    let ge_expr = Expression::BinaryOp {
        op: ">=".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Field("y".to_string())),
    };
    assert_eq!(
        engine.evaluate_expression(&ge_expr).unwrap(),
        FieldValue::Boolean(false)
    );

    // Test ==
    let eq_expr = Expression::BinaryOp {
        op: "==".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Number(10.0)),
    };
    assert_eq!(
        engine.evaluate_expression(&eq_expr).unwrap(),
        FieldValue::Boolean(true)
    );

    // Test !=
    let ne_expr = Expression::BinaryOp {
        op: "!=".to_string(),
        left: Box::new(Expression::Field("x".to_string())),
        right: Box::new(Expression::Field("y".to_string())),
    };
    assert_eq!(
        engine.evaluate_expression(&ne_expr).unwrap(),
        FieldValue::Boolean(true)
    );
}

#[test]
fn test_string_operations() {
    let mut engine = CalculationEngine::new();

    engine.set_field_value("first", FieldValue::Text("Hello".to_string()));
    engine.set_field_value("second", FieldValue::Text(" World".to_string()));

    // Test concatenación
    let concat_expr = Expression::BinaryOp {
        op: "+".to_string(),
        left: Box::new(Expression::Field("first".to_string())),
        right: Box::new(Expression::Field("second".to_string())),
    };

    let result = engine.evaluate_expression(&concat_expr);
    assert!(result.is_ok());
    // Nota: Dependiendo de la implementación, puede concatenar strings o convertir a números
}

#[test]
fn test_parse_expression_errors() {
    // Test expresiones inválidas
    let empty = "";
    let result = Expression::parse(empty);
    assert!(result.is_err());

    let invalid_syntax = "((( invalid";
    let result = Expression::parse(invalid_syntax);
    assert!(result.is_err());

    let unmatched_parens = "(a + b";
    let result = Expression::parse(unmatched_parens);
    assert!(result.is_err());
}

#[test]
fn test_circular_dependency_detection() {
    let mut engine = CalculationEngine::new();

    // Crear dependencia circular: A -> B -> C -> A
    let expr_a = Expression::Field("B".to_string());
    let expr_b = Expression::Field("C".to_string());
    let expr_c = Expression::Field("A".to_string());

    engine.add_calculation("A", expr_a);
    engine.add_calculation("B", expr_b);
    let result = engine.add_calculation("C", expr_c);

    // Debería detectar dependencia circular
    assert!(result.is_err());
}

#[test]
fn test_complex_nested_expressions() {
    let mut engine = CalculationEngine::new();

    engine.set_field_value("base", FieldValue::Number(100.0));
    engine.set_field_value("rate", FieldValue::Number(0.1));
    engine.set_field_value("years", FieldValue::Number(5.0));

    // Test expresión compleja: base * (1 + rate) ^ years
    let complex_expr = Expression::BinaryOp {
        op: "*".to_string(),
        left: Box::new(Expression::Field("base".to_string())),
        right: Box::new(Expression::BinaryOp {
            op: "^".to_string(),
            left: Box::new(Expression::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expression::Number(1.0)),
                right: Box::new(Expression::Field("rate".to_string())),
            }),
            right: Box::new(Expression::Field("years".to_string())),
        }),
    };

    let result = engine.evaluate_expression(&complex_expr);
    assert!(result.is_ok());
    // El resultado debería ser aproximadamente 161.05
    if let FieldValue::Number(n) = result.unwrap() {
        assert!((n - 161.05).abs() < 0.1);
    }
}

#[test]
fn test_field_array_operations() {
    let mut engine = CalculationEngine::new();

    // Test con array de campos
    engine.set_field_value("item.0", FieldValue::Number(10.0));
    engine.set_field_value("item.1", FieldValue::Number(20.0));
    engine.set_field_value("item.2", FieldValue::Number(30.0));

    // Test acceso a elementos del array
    let array_expr = Expression::Field("item.0".to_string());
    let result = engine.evaluate_expression(&array_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Number(10.0));
}

#[test]
fn test_validation_rules() {
    let mut engine = CalculationEngine::new();

    // Test regla de validación: valor debe estar entre 0 y 100
    engine.set_field_value("score", FieldValue::Number(85.0));

    let validation_expr = Expression::BinaryOp {
        op: "&&".to_string(),
        left: Box::new(Expression::BinaryOp {
            op: ">=".to_string(),
            left: Box::new(Expression::Field("score".to_string())),
            right: Box::new(Expression::Number(0.0)),
        }),
        right: Box::new(Expression::BinaryOp {
            op: "<=".to_string(),
            left: Box::new(Expression::Field("score".to_string())),
            right: Box::new(Expression::Number(100.0)),
        }),
    };

    let result = engine.evaluate_expression(&validation_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Boolean(true));

    // Test con valor inválido
    engine.set_field_value("score", FieldValue::Number(150.0));
    let result = engine.evaluate_expression(&validation_expr);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FieldValue::Boolean(false));
}

#[test]
fn test_expression_with_whitespace() {
    // Test parsing con espacios en blanco
    let expr_with_spaces = "  ( a  +  b )  *  c  ";
    let result = Expression::parse(expr_with_spaces);
    assert!(result.is_ok());

    let expr_with_tabs = "\ta\t+\tb\t";
    let result = Expression::parse(expr_with_tabs);
    assert!(result.is_ok());

    let expr_with_newlines = "a +\n b +\n c";
    let result = Expression::parse(expr_with_newlines);
    assert!(result.is_ok());
}

#[test]
fn test_special_numeric_values() {
    let mut engine = CalculationEngine::new();

    // Test con valores especiales
    engine.set_field_value("inf", FieldValue::Number(f64::INFINITY));
    engine.set_field_value("neg_inf", FieldValue::Number(f64::NEG_INFINITY));
    engine.set_field_value("nan", FieldValue::Number(f64::NAN));

    // Operaciones con infinito
    let inf_add = Expression::BinaryOp {
        op: "+".to_string(),
        left: Box::new(Expression::Field("inf".to_string())),
        right: Box::new(Expression::Number(100.0)),
    };

    let result = engine.evaluate_expression(&inf_add);
    assert!(result.is_ok());
    if let FieldValue::Number(n) = result.unwrap() {
        assert!(n.is_infinite());
    }

    // Operaciones con NaN
    let nan_op = Expression::BinaryOp {
        op: "*".to_string(),
        left: Box::new(Expression::Field("nan".to_string())),
        right: Box::new(Expression::Number(10.0)),
    };

    let result = engine.evaluate_expression(&nan_op);
    assert!(result.is_ok());
    if let FieldValue::Number(n) = result.unwrap() {
        assert!(n.is_nan());
    }
}

#[test]
fn test_remove_calculation_cascading() {
    let mut engine = CalculationEngine::new();

    // Crear cadena de dependencias: A -> B -> C
    engine.add_calculation("C", Expression::Number(10.0));
    engine.add_calculation("B", Expression::Field("C".to_string()));
    engine.add_calculation("A", Expression::Field("B".to_string()));

    // Remover B debería actualizar las dependencias
    engine.remove_calculation("B");

    // A ya no debería poder calcularse correctamente
    let result = engine.calculate("A");
    assert!(result.is_err() || result.unwrap() == FieldValue::Null);
}

#[test]
fn test_batch_calculation_updates() {
    let mut engine = CalculationEngine::new();

    // Configurar múltiples campos interdependientes
    engine.set_field_value("base", FieldValue::Number(100.0));
    engine.add_calculation(
        "tax",
        Expression::BinaryOp {
            op: "*".to_string(),
            left: Box::new(Expression::Field("base".to_string())),
            right: Box::new(Expression::Number(0.15)),
        },
    );
    engine.add_calculation(
        "total",
        Expression::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expression::Field("base".to_string())),
            right: Box::new(Expression::Field("tax".to_string())),
        },
    );

    // Calcular todo
    engine.calculate_all();

    assert_eq!(
        *engine.get_field_value("tax").unwrap(),
        FieldValue::Number(15.0)
    );
    assert_eq!(
        *engine.get_field_value("total").unwrap(),
        FieldValue::Number(115.0)
    );

    // Actualizar base y recalcular
    engine.set_field_value("base", FieldValue::Number(200.0));
    engine.calculate_all();

    assert_eq!(
        *engine.get_field_value("tax").unwrap(),
        FieldValue::Number(30.0)
    );
    assert_eq!(
        *engine.get_field_value("total").unwrap(),
        FieldValue::Number(230.0)
    );
}
