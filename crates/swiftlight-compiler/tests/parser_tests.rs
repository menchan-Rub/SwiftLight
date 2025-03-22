use swiftlight_compiler::frontend::{
    lexer::Lexer,
    parser::{Parser, ast::{Expression, Statement, Declaration, Type}},
    error::SourceLocation,
};
use swiftlight_compiler::frontend::parser::{self, ast};
use swiftlight_compiler::frontend::source_map::SourceMap;

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

#[test]
fn test_parse_variable_declaration() {
    let source = "let x: Int = 42;";
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let result = parser::parse(file_id, source, &source_map);
    assert!(result.is_ok(), "パースエラー: {:?}", result.err());
    
    let ast = result.unwrap();
    
    // 変数宣言を検証
    if let Some(ast::Stmt::VarDecl(var_decl)) = ast.items.first() {
        assert_eq!(var_decl.name.name, "x");
        
        // 型アノテーションを検証
        if let Some(type_ann) = &var_decl.type_ann {
            if let ast::Type::Named(named_type) = &**type_ann {
                assert_eq!(named_type.name.name, "Int");
            } else {
                panic!("期待された型アノテーションではありません");
            }
        } else {
            panic!("型アノテーションがありません");
        }
        
        // 初期化式を検証
        if let Some(ast::Expr::Literal(ast::Literal::Integer(value))) = &var_decl.initializer {
            assert_eq!(*value, 42);
        } else {
            panic!("期待された初期化式ではありません");
        }
    } else {
        panic!("変数宣言がパースされていません");
    }
}

#[test]
fn test_parse_function_declaration() {
    let source = "fn add(a: Int, b: Int) -> Int { return a + b; }";
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let result = parser::parse(file_id, source, &source_map);
    assert!(result.is_ok(), "パースエラー: {:?}", result.err());
    
    let ast = result.unwrap();
    
    // 関数宣言を検証
    if let Some(ast::Stmt::FnDecl(fn_decl)) = ast.items.first() {
        assert_eq!(fn_decl.name.name, "add");
        
        // パラメータを検証
        assert_eq!(fn_decl.params.len(), 2);
        assert_eq!(fn_decl.params[0].name.name, "a");
        assert_eq!(fn_decl.params[1].name.name, "b");
        
        // 戻り値の型を検証
        if let Some(return_type) = &fn_decl.return_type {
            if let ast::Type::Named(named_type) = &**return_type {
                assert_eq!(named_type.name.name, "Int");
            } else {
                panic!("期待された戻り値の型ではありません");
            }
        } else {
            panic!("戻り値の型がありません");
        }
        
        // 関数本体を検証
        assert_eq!(fn_decl.body.stmts.len(), 1);
        if let ast::Stmt::Return(return_stmt) = &fn_decl.body.stmts[0] {
            if let Some(ast::Expr::Binary(binary_expr)) = &return_stmt.value {
                assert_eq!(binary_expr.op, ast::BinaryOp::Add);
                
                if let ast::Expr::Ident(left_ident) = &*binary_expr.left {
                    assert_eq!(left_ident.name, "a");
                } else {
                    panic!("期待された左辺の式ではありません");
                }
                
                if let ast::Expr::Ident(right_ident) = &*binary_expr.right {
                    assert_eq!(right_ident.name, "b");
                } else {
                    panic!("期待された右辺の式ではありません");
                }
            } else {
                panic!("期待された二項演算式ではありません");
            }
        } else {
            panic!("期待されたreturn文ではありません");
        }
    } else {
        panic!("関数宣言がパースされていません");
    }
}

#[test]
fn test_parse_if_statement() {
    let source = "
        fn test() {
            if x > 10 {
                println(\"Large\");
            } else if x > 5 {
                println(\"Medium\");
            } else {
                println(\"Small\");
            }
        }
    ";
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let result = parser::parse(file_id, source, &source_map);
    assert!(result.is_ok(), "パースエラー: {:?}", result.err());
    
    let ast = result.unwrap();
    
    // 関数宣言を検証
    if let Some(ast::Stmt::FnDecl(fn_decl)) = ast.items.first() {
        assert_eq!(fn_decl.name.name, "test");
        
        // if文を検証
        assert_eq!(fn_decl.body.stmts.len(), 1);
        if let ast::Stmt::If(if_stmt) = &fn_decl.body.stmts[0] {
            // 条件式を検証
            if let ast::Expr::Binary(cond_expr) = &*if_stmt.condition {
                assert_eq!(cond_expr.op, ast::BinaryOp::Gt);
            } else {
                panic!("期待された条件式ではありません");
            }
            
            // else-if部分を検証
            if let Some(ast::Stmt::If(else_if_stmt)) = if_stmt.else_branch.as_ref().map(|b| &**b) {
                if let ast::Expr::Binary(cond_expr) = &*else_if_stmt.condition {
                    assert_eq!(cond_expr.op, ast::BinaryOp::Gt);
                } else {
                    panic!("期待されたelse-if条件式ではありません");
                }
                
                // else部分を検証
                assert!(else_if_stmt.else_branch.is_some());
            } else {
                panic!("期待されたelse-if文ではありません");
            }
        } else {
            panic!("期待されたif文ではありません");
        }
    } else {
        panic!("関数宣言がパースされていません");
    }
}

#[test]
fn test_parse_for_loop() {
    let source = "
        fn test() {
            for i in 0..10 {
                println(i);
            }
        }
    ";
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let result = parser::parse(file_id, source, &source_map);
    assert!(result.is_ok(), "パースエラー: {:?}", result.err());
    
    let ast = result.unwrap();
    
    // for文を検証
    if let Some(ast::Stmt::FnDecl(fn_decl)) = ast.items.first() {
        assert_eq!(fn_decl.body.stmts.len(), 1);
        if let ast::Stmt::ForIn(for_stmt) = &fn_decl.body.stmts[0] {
            assert_eq!(for_stmt.binding.name, "i");
            
            // イテレータ式を検証
            if let ast::Expr::Range(range_expr) = &*for_stmt.iterator {
                if let ast::Expr::Literal(ast::Literal::Integer(start)) = &*range_expr.start {
                    assert_eq!(*start, 0);
                } else {
                    panic!("期待された範囲開始式ではありません");
                }
                
                if let ast::Expr::Literal(ast::Literal::Integer(end)) = &*range_expr.end {
                    assert_eq!(*end, 10);
                } else {
                    panic!("期待された範囲終了式ではありません");
                }
            } else {
                panic!("期待された範囲式ではありません");
            }
            
            // 本体を検証
            assert_eq!(for_stmt.body.stmts.len(), 1);
        } else {
            panic!("期待されたfor文ではありません");
        }
    } else {
        panic!("関数宣言がパースされていません");
    }
}

#[test]
fn test_parse_error_recovery() {
    let source = "
        fn test() {
            let x = 10;
            let y = ;  // 構文エラー
            let z = 30;
        }
    ";
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let result = parser::parse(file_id, source, &source_map);
    
    // エラーを含むが、リカバリーしてパースは続行できるはず
    assert!(result.is_ok(), "パースエラーリカバリーに失敗: {:?}", result.err());
    
    let ast = result.unwrap();
    let diagnostics = ast.diagnostics;
    
    // エラーが検出されているはず
    assert!(!diagnostics.is_empty(), "エラーが検出されていません");
    
    // 関数宣言の本体を検証
    if let Some(ast::Stmt::FnDecl(fn_decl)) = ast.items.first() {
        // xとzの宣言は正しく解析されているはず
        assert!(fn_decl.body.stmts.len() >= 2, "リカバリー後の文の数が少なすぎます");
        
        // 最初の変数宣言を検証
        if let ast::Stmt::VarDecl(var_decl) = &fn_decl.body.stmts[0] {
            assert_eq!(var_decl.name.name, "x");
        } else {
            panic!("最初の変数宣言がパースされていません");
        }
        
        // 最後の変数宣言を検証（インデックスは実装によって異なる可能性がある）
        let last_stmt_idx = fn_decl.body.stmts.len() - 1;
        if let ast::Stmt::VarDecl(var_decl) = &fn_decl.body.stmts[last_stmt_idx] {
            assert_eq!(var_decl.name.name, "z");
        } else {
            panic!("最後の変数宣言がパースされていません");
        }
    } else {
        panic!("関数宣言がパースされていません");
    }
}
