use swiftlight_compiler::{
    frontend::parser::context_parser::{ContextParser, CompletionContextKind},
};

#[test]
fn test_empty_context() {
    let source = "";
    let mut parser = ContextParser::new(source, 0);
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::Empty => {
            // 期待通り
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_member_access_context() {
    let source = "let str = \"hello\";\nstr.";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::MemberAccess { expr, .. } => {
            assert_eq!(expr, "str");
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_static_access_context() {
    let source = "String::";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::StaticAccess { type_name } => {
            assert_eq!(type_name, "String");
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_type_annotation_context() {
    let source = "let x: ";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::TypeAnnotation { current_input, .. } => {
            assert!(current_input.is_none());
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_type_annotation_with_partial_input() {
    let source = "let x: In";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::TypeAnnotation { current_input, .. } => {
            assert_eq!(current_input.unwrap(), "In");
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_import_context() {
    let source = "import std:";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::Import { path_prefix } => {
            assert_eq!(path_prefix[0], "std");
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_function_argument_context() {
    let source = "print(";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::FunctionArgument { function_name, arg_index } => {
            assert_eq!(function_name, "print");
            assert_eq!(arg_index, 0);
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_function_argument_with_multiple_args() {
    let source = "max(10, ";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::FunctionArgument { function_name, arg_index } => {
            assert_eq!(function_name, "max");
            assert_eq!(arg_index, 1);
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_block_context() {
    let source = "func test() {\n    ";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::BlockStatement { block_kind, .. } => {
            assert_eq!(block_kind, "function");
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
}

#[test]
fn test_normal_context_with_visible_locals() {
    let source = "func test() {\n    let x = 10;\n    let y = 20;\n    ";
    let mut parser = ContextParser::new(source, source.len());
    let context = parser.analyze_context().unwrap();
    
    match context.kind {
        CompletionContextKind::BlockStatement { local_variables, .. } => {
            assert!(local_variables.contains(&"x".to_string()));
            assert!(local_variables.contains(&"y".to_string()));
        },
        _ => panic!("期待されたコンテキストではありません: {:?}", context.kind),
    }
} 