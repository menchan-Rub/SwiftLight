use swiftlight_compiler::frontend::{
    lexer::Lexer,
    parser::{Parser, ast::{Expression, Statement, Declaration, Type}},
    error::SourceLocation,
};

#[test]
fn test_parser_basic_expressions() {
    let source = "1 + 2 * 3";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let expr = parser.parse_expression().unwrap();
    
    // 期待される構造: Add(1, Multiply(2, 3))
    match expr {
        Expression::Binary { 
            left, 
            operator, 
            right 
        } => {
            assert_eq!(operator.to_string(), "+");
            
            // 左辺は整数リテラル1であること
            match *left {
                Expression::IntLiteral(val) => assert_eq!(val, 1),
                _ => panic!("Expected IntLiteral(1), got: {:?}", left),
            }
            
            // 右辺は2*3の乗算であること
            match *right {
                Expression::Binary {
                    left,
                    operator,
                    right,
                } => {
                    assert_eq!(operator.to_string(), "*");
                    
                    match *left {
                        Expression::IntLiteral(val) => assert_eq!(val, 2),
                        _ => panic!("Expected IntLiteral(2), got: {:?}", left),
                    }
                    
                    match *right {
                        Expression::IntLiteral(val) => assert_eq!(val, 3),
                        _ => panic!("Expected IntLiteral(3), got: {:?}", right),
                    }
                },
                _ => panic!("Expected Binary operation, got: {:?}", right),
            }
        },
        _ => panic!("Expected Binary expression, got: {:?}", expr),
    }
}

#[test]
fn test_parser_variable_declaration() {
    let source = "let x: i32 = 42;";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let stmt = &program.declarations[0];
    
    match stmt {
        Declaration::Variable { 
            name, 
            type_annotation, 
            initializer 
        } => {
            assert_eq!(*name, "x");
            
            // 型アノテーションの検証
            match type_annotation.as_ref().unwrap() {
                Type::Named(type_name) => assert_eq!(*type_name, "i32"),
                _ => panic!("Expected named type, got: {:?}", type_annotation),
            }
            
            // 初期化式の検証
            match initializer.as_ref().unwrap() {
                Expression::IntLiteral(val) => assert_eq!(*val, 42),
                _ => panic!("Expected IntLiteral(42), got: {:?}", initializer),
            }
        },
        _ => panic!("Expected Variable declaration, got: {:?}", stmt),
    }
}

#[test]
fn test_parser_function_declaration() {
    let source = "fn add(a: i32, b: i32) -> i32 { return a + b; }";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let decl = &program.declarations[0];
    
    match decl {
        Declaration::Function { 
            name, 
            parameters, 
            return_type, 
            body 
        } => {
            assert_eq!(*name, "add");
            
            // パラメータの検証
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].name, "a");
            assert_eq!(parameters[1].name, "b");
            
            // 戻り値の型の検証
            match return_type.as_ref().unwrap() {
                Type::Named(type_name) => assert_eq!(*type_name, "i32"),
                _ => panic!("Expected named type, got: {:?}", return_type),
            }
            
            // 関数本体に含まれるステートメント数の検証
            assert_eq!(body.statements.len(), 1);
        },
        _ => panic!("Expected Function declaration, got: {:?}", decl),
    }
}

#[test]
fn test_parser_if_statement() {
    let source = "fn test() { if x > 0 { let y = 10; } else { let y = 20; } }";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let function = &program.declarations[0];
    
    match function {
        Declaration::Function { body, .. } => {
            let stmt = &body.statements[0];
            
            match stmt {
                Statement::If { 
                    condition, 
                    then_branch, 
                    else_branch 
                } => {
                    // 条件式の検証
                    match condition {
                        Expression::Binary { 
                            left, 
                            operator, 
                            right 
                        } => {
                            assert_eq!(operator.to_string(), ">");
                            
                            match left.as_ref() {
                                Expression::Identifier(name) => assert_eq!(*name, "x"),
                                _ => panic!("Expected Identifier, got: {:?}", left),
                            }
                            
                            match right.as_ref() {
                                Expression::IntLiteral(val) => assert_eq!(*val, 0),
                                _ => panic!("Expected IntLiteral(0), got: {:?}", right),
                            }
                        },
                        _ => panic!("Expected Binary expression, got: {:?}", condition),
                    }
                    
                    // then ブロックの検証
                    assert_eq!(then_branch.statements.len(), 1);
                    
                    // else ブロックの検証
                    assert!(else_branch.is_some());
                    assert_eq!(else_branch.as_ref().unwrap().statements.len(), 1);
                },
                _ => panic!("Expected If statement, got: {:?}", stmt),
            }
        },
        _ => panic!("Expected Function declaration"),
    }
}

#[test]
fn test_parser_error_recovery() {
    let source = "fn test() { let x = 1 + ; return x; }";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let decl = &program.declarations[0];
    
    // エラーリカバリーにより、パースは成功するはず
    match decl {
        Declaration::Function { 
            name, 
            body, 
            .. 
        } => {
            assert_eq!(*name, "test");
            
            // エラーがあっても関数本体をパースできているか
            assert!(body.statements.len() >= 1);
            
            // パーサーのエラーカウントを検証
            assert!(parser.error_count() > 0);
        },
        _ => panic!("Expected Function declaration, got: {:?}", decl),
    }
}

#[test]
fn test_parser_struct_declaration() {
    let source = "struct Point { x: f64, y: f64 }";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let decl = &program.declarations[0];
    
    match decl {
        Declaration::Struct { 
            name, 
            fields 
        } => {
            assert_eq!(*name, "Point");
            
            // フィールドの検証
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
            
            // フィールドの型の検証
            match &fields[0].type_annotation {
                Type::Named(type_name) => assert_eq!(*type_name, "f64"),
                _ => panic!("Expected named type, got: {:?}", fields[0].type_annotation),
            }
            
            match &fields[1].type_annotation {
                Type::Named(type_name) => assert_eq!(*type_name, "f64"),
                _ => panic!("Expected named type, got: {:?}", fields[1].type_annotation),
            }
        },
        _ => panic!("Expected Struct declaration, got: {:?}", decl),
    }
}

#[test]
fn test_parser_trait_declaration() {
    let source = "trait Printable { fn print(&self); fn debug_print(&self) -> String; }";
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    
    let program = parser.parse().unwrap();
    let decl = &program.declarations[0];
    
    match decl {
        Declaration::Trait { 
            name, 
            methods 
        } => {
            assert_eq!(*name, "Printable");
            
            // メソッドの検証
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "print");
            assert_eq!(methods[1].name, "debug_print");
            
            // メソッドのシグネチャ検証
            assert!(methods[0].return_type.is_none());
            assert!(methods[1].return_type.is_some());
        },
        _ => panic!("Expected Trait declaration, got: {:?}", decl),
    }
}
