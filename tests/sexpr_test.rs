//! S表达式解析器集成测试
//!
//! 测试完整的S表达式解析和转换流程。

use equation_compiler::sexpr::{parse, parse_to_expr, parse_to_yaml, to_yaml_value};
use equation_compiler::ast::Expr;
use std::fs;

// ============================================
// 基础解析测试
// ============================================

#[test]
fn test_parse_number() {
    let sexpr = parse("42").unwrap();
    assert!(sexpr.is_number());
    assert_eq!(sexpr.as_number(), Some(42.0));
}

#[test]
fn test_parse_float() {
    let sexpr = parse("3.14159").unwrap();
    assert!(sexpr.is_number());
    #[allow(clippy::approx_constant)]
    let expected = 3.14159;
    assert!((sexpr.as_number().unwrap() - expected).abs() < 1e-10);
}

#[test]
fn test_parse_scientific() {
    let sexpr = parse("1.5e-10").unwrap();
    assert!(sexpr.is_number());
    assert!((sexpr.as_number().unwrap() - 1.5e-10).abs() < 1e-20);
}

#[test]
fn test_parse_negative() {
    let sexpr = parse("-42").unwrap();
    assert!(sexpr.is_number());
    assert_eq!(sexpr.as_number(), Some(-42.0));
}

#[test]
fn test_parse_symbol() {
    let sexpr = parse("x").unwrap();
    assert!(sexpr.is_symbol());
    assert_eq!(sexpr.as_symbol(), Some("x"));
}

#[test]
fn test_parse_complex_symbol() {
    let sexpr = parse("param_name_123").unwrap();
    assert!(sexpr.is_symbol());
    assert_eq!(sexpr.as_symbol(), Some("param_name_123"));
}

#[test]
fn test_parse_simple_list() {
    let sexpr = parse("(add 1 2)").unwrap();
    assert!(sexpr.is_list());
    let list = sexpr.as_list().unwrap();
    assert_eq!(list.len(), 3);
}

#[test]
fn test_parse_nested_list() {
    let sexpr = parse("(mul (add x 1) (sub y 2))").unwrap();
    assert!(sexpr.is_list());
    let list = sexpr.as_list().unwrap();
    assert_eq!(list.len(), 3);
    assert!(list[1].is_list());
    assert!(list[2].is_list());
}

#[test]
fn test_parse_with_comments() {
    let input = r#"
        ; This is a comment
        (add 1 2) ; inline comment
    "#;
    let sexpr = parse(input).unwrap();
    assert!(sexpr.is_list());
}

// ============================================
// 转换测试
// ============================================

#[test]
fn test_convert_number() {
    let expr = parse_to_expr("42").unwrap();
    assert!(matches!(expr, Expr::Const(n) if (n - 42.0).abs() < 1e-10));
}

#[test]
fn test_convert_variable() {
    let expr = parse_to_expr("x").unwrap();
    assert!(matches!(expr, Expr::Var(ref s) if s == "x"));
}

#[test]
fn test_convert_pi() {
    let expr = parse_to_expr("pi").unwrap();
    assert!(matches!(expr, Expr::Pi));
}

#[test]
fn test_convert_e() {
    let expr = parse_to_expr("e").unwrap();
    assert!(matches!(expr, Expr::E));
}

#[test]
fn test_convert_add() {
    let expr = parse_to_expr("(add 1 2)").unwrap();
    assert!(matches!(expr, Expr::Add(_, _)));
}

#[test]
fn test_convert_sub() {
    let expr = parse_to_expr("(sub 10 5)").unwrap();
    assert!(matches!(expr, Expr::Sub(_, _)));
}

#[test]
fn test_convert_mul() {
    let expr = parse_to_expr("(mul 3 4)").unwrap();
    assert!(matches!(expr, Expr::Mul(_, _)));
}

#[test]
fn test_convert_div() {
    let expr = parse_to_expr("(div 10 2)").unwrap();
    assert!(matches!(expr, Expr::Div(_, _)));
}

#[test]
fn test_convert_pow() {
    let expr = parse_to_expr("(pow x 2)").unwrap();
    assert!(matches!(expr, Expr::Pow(_, _)));
}

#[test]
fn test_convert_neg() {
    let expr = parse_to_expr("(neg x)").unwrap();
    assert!(matches!(expr, Expr::Neg(_)));
}

#[test]
fn test_convert_abs() {
    let expr = parse_to_expr("(abs x)").unwrap();
    assert!(matches!(expr, Expr::Abs(_)));
}

// ============================================
// 三角函数测试
// ============================================

#[test]
fn test_convert_trig() {
    let _ = parse_to_expr("(sin x)").unwrap();
    let _ = parse_to_expr("(cos x)").unwrap();
    let _ = parse_to_expr("(tan x)").unwrap();
    let _ = parse_to_expr("(asin x)").unwrap();
    let _ = parse_to_expr("(acos x)").unwrap();
    let _ = parse_to_expr("(atan x)").unwrap();
    let _ = parse_to_expr("(atan2 y x)").unwrap();
}

#[test]
fn test_convert_hyperbolic() {
    let _ = parse_to_expr("(sinh x)").unwrap();
    let _ = parse_to_expr("(cosh x)").unwrap();
    let _ = parse_to_expr("(tanh x)").unwrap();
    let _ = parse_to_expr("(asinh x)").unwrap();
    let _ = parse_to_expr("(acosh x)").unwrap();
    let _ = parse_to_expr("(atanh x)").unwrap();
}

// ============================================
// 特殊函数测试
// ============================================

#[test]
fn test_convert_special_functions() {
    let _ = parse_to_expr("(gamma x)").unwrap();
    let _ = parse_to_expr("(lgamma x)").unwrap();
    let _ = parse_to_expr("(digamma x)").unwrap();
    let _ = parse_to_expr("(beta a b)").unwrap();
    let _ = parse_to_expr("(erf x)").unwrap();
    let _ = parse_to_expr("(erfc x)").unwrap();
    let _ = parse_to_expr("(factorial 5)").unwrap();
    let _ = parse_to_expr("(combination 10 3)").unwrap();
}

#[test]
fn test_convert_bessel() {
    let _ = parse_to_expr("(bessel_j0 x)").unwrap();
    let _ = parse_to_expr("(bessel_j1 x)").unwrap();
    let _ = parse_to_expr("(bessel_jn 2 x)").unwrap();
    let _ = parse_to_expr("(bessel_y0 x)").unwrap();
    let _ = parse_to_expr("(bessel_y1 x)").unwrap();
    let _ = parse_to_expr("(bessel_yn 2 x)").unwrap();
}

// ============================================
// 条件表达式测试
// ============================================

#[test]
fn test_convert_if() {
    let expr = parse_to_expr("(if (gt x 0) (sqrt x) 0)").unwrap();
    assert!(matches!(expr, Expr::IfThenElse { .. }));
}

#[test]
fn test_convert_comparison() {
    let _ = parse_to_expr("(eq x y)").unwrap();
    let _ = parse_to_expr("(lt x y)").unwrap();
    let _ = parse_to_expr("(gt x y)").unwrap();
    let _ = parse_to_expr("(leq x y)").unwrap();
    let _ = parse_to_expr("(geq x y)").unwrap();
    let _ = parse_to_expr("(neq x y)").unwrap();
}

#[test]
fn test_convert_logical() {
    let _ = parse_to_expr("(and (gt x 0) (lt x 10))").unwrap();
    let _ = parse_to_expr("(or (lt x 0) (gt x 10))").unwrap();
    let _ = parse_to_expr("(not (eq x 0))").unwrap();
}

// ============================================
// 求和/连乘测试
// ============================================

#[test]
fn test_convert_sum() {
    let expr = parse_to_expr("(sum i 1 n (pow i 2))").unwrap();
    if let Expr::Sum { index, .. } = expr {
        assert_eq!(index, "i");
    } else {
        panic!("Expected Sum");
    }
}

#[test]
fn test_convert_product() {
    let expr = parse_to_expr("(product k 1 10 k)").unwrap();
    if let Expr::Product { index, .. } = expr {
        assert_eq!(index, "k");
    } else {
        panic!("Expected Product");
    }
}

// ============================================
// 分段函数测试
// ============================================

#[test]
fn test_convert_piecewise() {
    let expr = parse_to_expr("(piecewise ((lt x 0) (neg x)) :otherwise x)").unwrap();
    if let Expr::Piecewise { pieces, .. } = expr {
        assert_eq!(pieces.len(), 1);
    } else {
        panic!("Expected Piecewise");
    }
}

#[test]
fn test_convert_piecewise_multiple() {
    let expr = parse_to_expr(
        "(piecewise ((lt x 0) (neg x)) ((eq x 0) 0) :otherwise x)"
    ).unwrap();
    if let Expr::Piecewise { pieces, .. } = expr {
        assert_eq!(pieces.len(), 2);
    } else {
        panic!("Expected Piecewise");
    }
}

// ============================================
// 复杂表达式测试
// ============================================

#[test]
fn test_convert_complex_expression() {
    let input = r#"
        (div
            (add
                (mul 2 x)
                (pow y 2))
            (sub
                z
                1))
    "#;
    let expr = parse_to_expr(input).unwrap();
    assert!(matches!(expr, Expr::Div(_, _)));
}

#[test]
fn test_convert_quadratic_formula() {
    let input = "(div (add (neg b) (sqrt (sub (pow b 2) (mul 4 (mul a c))))) (mul 2 a))";
    let expr = parse_to_expr(input).unwrap();
    assert!(matches!(expr, Expr::Div(_, _)));
}

#[test]
fn test_convert_gaussian() {
    let input = "(mul (div 1 (mul sigma (sqrt (mul 2 pi)))) (exp (neg (div (pow (sub x mu) 2) (mul 2 (pow sigma 2))))))";
    let expr = parse_to_expr(input).unwrap();
    assert!(matches!(expr, Expr::Mul(_, _)));
}

// ============================================
// YAML序列化测试
// ============================================

#[test]
fn test_to_yaml_number() {
    let expr = Expr::Const(42.0);
    let yaml = to_yaml_value(&expr);
    assert!(yaml.is_mapping());
}

#[test]
fn test_to_yaml_add() {
    let expr = Expr::add(Expr::var("x"), Expr::Const(1.0));
    let yaml = to_yaml_value(&expr);
    let map = yaml.as_mapping().unwrap();
    assert_eq!(
        map.get(serde_yaml::Value::String("op".to_string())),
        Some(&serde_yaml::Value::String("add".to_string()))
    );
}

#[test]
fn test_parse_to_yaml() {
    let yaml = parse_to_yaml("(add x 1)").unwrap();
    assert!(yaml.is_mapping());
}

// ============================================
// 错误处理测试
// ============================================

#[test]
fn test_error_unknown_operator() {
    let result = parse_to_expr("(foobar x y)");
    assert!(result.is_err());
}

#[test]
fn test_error_wrong_arg_count() {
    let result = parse_to_expr("(add 1 2 3)");
    assert!(result.is_err());
}

#[test]
fn test_error_unmatched_paren() {
    let result = parse("(add 1 2");
    assert!(result.is_err());
}

#[test]
fn test_error_extra_paren() {
    let result = parse("add 1 2)");
    assert!(result.is_err());
}

// ============================================
// 别名测试
// ============================================

#[test]
fn test_operator_aliases() {
    // 三角函数别名
    let _ = parse_to_expr("(arcsin x)").unwrap();
    let _ = parse_to_expr("(arccos x)").unwrap();
    let _ = parse_to_expr("(arctan x)").unwrap();
    
    // 双曲函数别名
    let _ = parse_to_expr("(arcsinh x)").unwrap();
    let _ = parse_to_expr("(arccosh x)").unwrap();
    let _ = parse_to_expr("(arctanh x)").unwrap();
    
    // 对数别名
    let _ = parse_to_expr("(log x)").unwrap();  // 等同于 ln
    
    // 其他别名
    let _ = parse_to_expr("(rem 10 3)").unwrap();  // 等同于 mod
    let _ = parse_to_expr("(signum x)").unwrap();  // 等同于 sign
}

// ============================================
// 向量/矩阵测试
// ============================================

#[test]
fn test_convert_vector() {
    let expr = parse_to_expr("(vector 1 2 3)").unwrap();
    assert!(matches!(expr, Expr::VectorLit(_)));
}

#[test]
fn test_convert_dot() {
    let expr = parse_to_expr("(dot v1 v2)").unwrap();
    assert!(matches!(expr, Expr::Dot(_, _)));
}

#[test]
fn test_convert_cross() {
    let expr = parse_to_expr("(cross v1 v2)").unwrap();
    assert!(matches!(expr, Expr::Cross(_, _)));
}

// ============================================
// 聚合函数测试
// ============================================

#[test]
fn test_convert_max_min() {
    let _ = parse_to_expr("(max x y z)").unwrap();
    let _ = parse_to_expr("(min x y z)").unwrap();
}

// ============================================
// Lambda表达式测试
// ============================================

#[test]
fn test_convert_lambda() {
    let expr = parse_to_expr("(lambda x (add x 1))").unwrap();
    if let Expr::Lambda { var, .. } = expr {
        assert_eq!(var, "x");
    } else {
        panic!("Expected Lambda");
    }
}

// ============================================
// 概率分布测试
// ============================================

#[test]
fn test_convert_distributions() {
    let _ = parse_to_expr("(norm_pdf x 0 1)").unwrap();
    let _ = parse_to_expr("(norm_cdf x 0 1)").unwrap();
    let _ = parse_to_expr("(norm_ppf p 0 1)").unwrap();
    let _ = parse_to_expr("(t_pdf x 5)").unwrap();
    let _ = parse_to_expr("(t_cdf x 5)").unwrap();
    let _ = parse_to_expr("(chi2_pdf x 3)").unwrap();
    let _ = parse_to_expr("(chi2_cdf x 3)").unwrap();
}

// ============================================
// 往返测试 (Roundtrip Tests)
// ============================================

/// 验证 S-Expr -> Expr -> YAML 的一致性
#[test]
fn test_roundtrip_basic_arithmetic() {
    let expressions = vec![
        "(add x 1)",
        "(sub x y)",
        "(mul a b)",
        "(div x 2)",
        "(pow x 2)",
        "(neg x)",
        "(abs x)",
    ];
    
    for input in expressions {
        let expr = parse_to_expr(input).unwrap();
        let yaml = to_yaml_value(&expr);
        
        // 验证YAML结构正确
        assert!(yaml.is_mapping(), "YAML should be a mapping for: {}", input);
        
        let map = yaml.as_mapping().unwrap();
        assert!(
            map.contains_key(serde_yaml::Value::String("op".to_string())) ||
            map.contains_key(serde_yaml::Value::String("ref".to_string())) ||
            map.contains_key(serde_yaml::Value::String("const".to_string())),
            "YAML should have op/ref/const for: {}", input
        );
    }
}

#[test]
fn test_roundtrip_trigonometric() {
    let expressions = vec![
        "(sin x)",
        "(cos x)",
        "(tan x)",
        "(asin x)",
        "(acos x)",
        "(atan x)",
        "(sinh x)",
        "(cosh x)",
        "(tanh x)",
    ];
    
    for input in expressions {
        let expr = parse_to_expr(input).unwrap();
        let yaml = to_yaml_value(&expr);
        assert!(yaml.is_mapping(), "YAML should be mapping for: {}", input);
    }
}

#[test]
fn test_roundtrip_if_then_else() {
    let input = "(if (gt x 0) (sqrt x) 0)";
    let expr = parse_to_expr(input).unwrap();
    let yaml = to_yaml_value(&expr);
    
    let map = yaml.as_mapping().unwrap();
    assert!(map.contains_key(serde_yaml::Value::String("if".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("then".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("else".to_string())));
}

#[test]
fn test_roundtrip_sum() {
    let input = "(sum i 1 n (pow i 2))";
    let expr = parse_to_expr(input).unwrap();
    let yaml = to_yaml_value(&expr);
    
    let map = yaml.as_mapping().unwrap();
    assert!(map.contains_key(serde_yaml::Value::String("sum".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("lower".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("upper".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("body".to_string())));
}

#[test]
fn test_roundtrip_piecewise() {
    let input = "(piecewise ((lt x 0) (neg x)) :otherwise x)";
    let expr = parse_to_expr(input).unwrap();
    let yaml = to_yaml_value(&expr);
    
    let map = yaml.as_mapping().unwrap();
    assert!(map.contains_key(serde_yaml::Value::String("pieces".to_string())));
    assert!(map.contains_key(serde_yaml::Value::String("otherwise".to_string())));
}

#[test]
fn test_roundtrip_complex_expression() {
    let input = "(div (add (mul 2 x) (pow y 2)) (sub z 1))";
    let expr = parse_to_expr(input).unwrap();
    let yaml = to_yaml_value(&expr);
    
    // 验证可以序列化为字符串
    let yaml_str = serde_yaml::to_string(&yaml).unwrap();
    assert!(yaml_str.contains("div"));
}

// ============================================
// 端到端测试 (End-to-End Tests)
// ============================================

/// 测试从S表达式到Python代码生成
#[test]
fn test_e2e_python_codegen() {
    let input = "(add (mul 2 x) (pow y 2))";
    let expr = parse_to_expr(input).unwrap();
    
    // 生成Python代码
    let python = expr.to_python("");
    
    // 验证Python代码包含正确的结构
    assert!(python.contains("+") || python.contains("2"));
    assert!(python.contains("**") || python.contains("pow"));
}

/// 测试从S表达式到Rust代码生成
#[test]
fn test_e2e_rust_codegen() {
    let input = "(add (mul 2 x) (pow y 2))";
    let expr = parse_to_expr(input).unwrap();
    
    // 生成Rust代码
    let rust = expr.to_rust();
    
    // 验证Rust代码包含正确的结构
    assert!(rust.contains("+") || rust.contains("*"));
    assert!(rust.contains("powi") || rust.contains("powf"));
}

/// 测试从S表达式到LaTeX生成
#[test]
fn test_e2e_latex_codegen() {
    let input = "(div (add x y) (mul 2 z))";
    let expr = parse_to_expr(input).unwrap();
    
    // 生成LaTeX
    let latex = expr.to_latex();
    
    // 验证LaTeX包含分数
    assert!(latex.contains("frac") || latex.contains("/"));
}

/// 测试条件表达式代码生成
#[test]
fn test_e2e_conditional_codegen() {
    let input = "(if (gt x 0) x (neg x))";
    let expr = parse_to_expr(input).unwrap();
    
    let python = expr.to_python("");
    let rust = expr.to_rust();
    
    // 验证条件结构
    assert!(python.contains("if") || python.contains("where"));
    assert!(rust.contains("if") || rust.contains("else"));
}

/// 测试求和表达式代码生成
#[test]
fn test_e2e_sum_codegen() {
    let input = "(sum i 1 10 (pow i 2))";
    let expr = parse_to_expr(input).unwrap();
    
    let python = expr.to_python("");
    let rust = expr.to_rust();
    
    // 验证求和结构
    assert!(python.contains("sum") || python.contains("for"));
    assert!(rust.contains("iter") || rust.contains("sum") || rust.contains("fold"));
}

/// 测试特殊函数代码生成
#[test]
fn test_e2e_special_functions_codegen() {
    let expressions = vec![
        "(gamma x)",
        "(erf x)",
        "(bessel_j0 x)",
    ];
    
    for input in expressions {
        let expr = parse_to_expr(input).unwrap();
        
        // Python代码应该引用scipy.special
        let python = expr.to_python("");
        assert!(!python.is_empty(), "Python code should not be empty for: {}", input);
        
        // Rust代码应该包含函数调用
        let rust = expr.to_rust();
        assert!(!rust.is_empty(), "Rust code should not be empty for: {}", input);
    }
}

// ============================================
// 样例文件测试
// ============================================

/// 测试解析样例文件（解析完整文件）
#[test]
fn test_parse_sample_files() {
    let sample_dir = "tests/sexpr_samples";
    
    // 检查目录是否存在
    if !std::path::Path::new(sample_dir).exists() {
        return; // 跳过如果目录不存在
    }
    
    // 测试可以作为单独表达式解析的简单样例
    let simple_expressions = vec![
        "(add 1 2)",
        "(sin x)",
        "(gamma x)",
        "(norm_pdf x 0 1)",
    ];
    
    for expr_str in simple_expressions {
        let result = parse(expr_str);
        assert!(result.is_ok(), "Failed to parse: {}", expr_str);
    }
    
    // 测试文件中的单行表达式
    let basic_file = format!("{}/basic.sexpr", sample_dir);
    if std::path::Path::new(&basic_file).exists() {
        let content = fs::read_to_string(&basic_file).unwrap();
        
        // 统计可解析的表达式数量
        let mut parsed_count = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            
            // 只解析完整的单行表达式
            let paren_balance: i32 = line.chars()
                .map(|c| match c { '(' => 1, ')' => -1, _ => 0 })
                .sum();
            
            if paren_balance == 0 && !line.is_empty() {
                let result = parse(line);
                if result.is_ok() {
                    parsed_count += 1;
                }
            }
        }
        
        // 应该能够解析一些表达式
        assert!(parsed_count > 0, "Should parse some expressions from basic.sexpr");
    }
}

// ============================================
// 新增运算符测试 (扩展覆盖)
// ============================================

#[test]
fn test_convert_jacobi_elliptic() {
    let _ = parse_to_expr("(jacobi_sn u m)").unwrap();
    let _ = parse_to_expr("(jacobi_cn u m)").unwrap();
    let _ = parse_to_expr("(jacobi_dn u m)").unwrap();
    let _ = parse_to_expr("(sn u m)").unwrap();
    let _ = parse_to_expr("(cn u m)").unwrap();
    let _ = parse_to_expr("(dn u m)").unwrap();
}

#[test]
fn test_convert_gegenbauer_jacobi_p() {
    let _ = parse_to_expr("(gegenbauer n alpha x)").unwrap();
    let _ = parse_to_expr("(ultraspherical n alpha x)").unwrap();
    let _ = parse_to_expr("(jacobi_p n alpha beta x)").unwrap();
}

#[test]
fn test_convert_theta_functions() {
    let _ = parse_to_expr("(theta1 z q)").unwrap();
    let _ = parse_to_expr("(theta2 z q)").unwrap();
    let _ = parse_to_expr("(theta3 z q)").unwrap();
    let _ = parse_to_expr("(theta4 z q)").unwrap();
}

#[test]
fn test_convert_parabolic_cylinder() {
    let _ = parse_to_expr("(pbdv v x)").unwrap();
    let _ = parse_to_expr("(pbvv v x)").unwrap();
    let _ = parse_to_expr("(pbwa a x)").unwrap();
}

#[test]
fn test_convert_mathieu() {
    let _ = parse_to_expr("(mathieu_a m q)").unwrap();
    let _ = parse_to_expr("(mathieu_b m q)").unwrap();
    let _ = parse_to_expr("(mathieu_ce m q x)").unwrap();
    let _ = parse_to_expr("(mathieu_se m q x)").unwrap();
}

#[test]
fn test_convert_coulomb() {
    let _ = parse_to_expr("(coulomb_f l eta rho)").unwrap();
    let _ = parse_to_expr("(coulomb_g l eta rho)").unwrap();
}

#[test]
fn test_convert_wigner_symbols() {
    let _ = parse_to_expr("(wigner_3j j1 j2 j3 m1 m2 m3)").unwrap();
    let _ = parse_to_expr("(wigner_6j j1 j2 j3 j4 j5 j6)").unwrap();
    let _ = parse_to_expr("(wigner_9j j1 j2 j3 j4 j5 j6 j7 j8 j9)").unwrap();
}

#[test]
fn test_convert_spheroidal() {
    let _ = parse_to_expr("(pro_ang1 m n c x)").unwrap();
    let _ = parse_to_expr("(pro_rad1 m n c x)").unwrap();
    let _ = parse_to_expr("(obl_ang1 m n c x)").unwrap();
    let _ = parse_to_expr("(obl_rad1 m n c x)").unwrap();
}

#[test]
fn test_convert_wright_voigt() {
    let _ = parse_to_expr("(wright_bessel a b x)").unwrap();
    let _ = parse_to_expr("(wright_omega z)").unwrap();
    let _ = parse_to_expr("(voigt sigma gamma x)").unwrap();
}

#[test]
fn test_convert_sigmoid_boxcox() {
    let _ = parse_to_expr("(logit p)").unwrap();
    let _ = parse_to_expr("(expit x)").unwrap();
    let _ = parse_to_expr("(sigmoid x)").unwrap();
    let _ = parse_to_expr("(boxcox y lmbda)").unwrap();
    let _ = parse_to_expr("(inv_boxcox y lmbda)").unwrap();
}

#[test]
fn test_convert_information_theory() {
    let _ = parse_to_expr("(entr x)").unwrap();
    let _ = parse_to_expr("(rel_entr x y)").unwrap();
    let _ = parse_to_expr("(kl_div p q)").unwrap();
}

#[test]
fn test_convert_factorial_extensions() {
    let _ = parse_to_expr("(factorial2 n)").unwrap();
    let _ = parse_to_expr("(factorialk n k)").unwrap();
    let _ = parse_to_expr("(stirling2 n k)").unwrap();
    let _ = parse_to_expr("(poch a x)").unwrap();
}

#[test]
fn test_convert_carlson_elliptic() {
    let _ = parse_to_expr("(elliprc x y)").unwrap();
    let _ = parse_to_expr("(elliprd x y z)").unwrap();
    let _ = parse_to_expr("(elliprf x y z)").unwrap();
    let _ = parse_to_expr("(elliprg x y z)").unwrap();
    let _ = parse_to_expr("(elliprj x y z p)").unwrap();
}

#[test]
fn test_convert_extended_error() {
    let _ = parse_to_expr("(erfcx x)").unwrap();
    let _ = parse_to_expr("(erfi x)").unwrap();
    let _ = parse_to_expr("(erfcinv p)").unwrap();
}

#[test]
fn test_convert_extended_gamma() {
    let _ = parse_to_expr("(hyperu a b x)").unwrap();
    let _ = parse_to_expr("(rgamma x)").unwrap();
    let _ = parse_to_expr("(gammasgn x)").unwrap();
}

#[test]
fn test_convert_convenience() {
    let _ = parse_to_expr("(agm a b)").unwrap();
    let _ = parse_to_expr("(exprel x)").unwrap();
    let _ = parse_to_expr("(xlogy x y)").unwrap();
    let _ = parse_to_expr("(xlog1py x y)").unwrap();
}

#[test]
fn test_convert_zeta_extensions() {
    let _ = parse_to_expr("(hurwitz_zeta s a)").unwrap();
    let _ = parse_to_expr("(zetac s)").unwrap();
    let _ = parse_to_expr("(polylog n z)").unwrap();
}

#[test]
fn test_convert_scaled_bessel() {
    let _ = parse_to_expr("(i0e x)").unwrap();
    let _ = parse_to_expr("(i1e x)").unwrap();
    let _ = parse_to_expr("(ive v x)").unwrap();
    let _ = parse_to_expr("(k0e x)").unwrap();
    let _ = parse_to_expr("(k1e x)").unwrap();
    let _ = parse_to_expr("(kve v x)").unwrap();
    let _ = parse_to_expr("(jve v x)").unwrap();
    let _ = parse_to_expr("(yve v x)").unwrap();
    let _ = parse_to_expr("(hankel1e v x)").unwrap();
    let _ = parse_to_expr("(hankel2e v x)").unwrap();
}

#[test]
fn test_convert_bessel_derivatives() {
    let _ = parse_to_expr("(jvp v x)").unwrap();
    let _ = parse_to_expr("(yvp v x)").unwrap();
    let _ = parse_to_expr("(ivp v x)").unwrap();
    let _ = parse_to_expr("(kvp v x)").unwrap();
    let _ = parse_to_expr("(h1vp v x)").unwrap();
    let _ = parse_to_expr("(h2vp v x)").unwrap();
}

#[test]
fn test_convert_huber_loss() {
    let _ = parse_to_expr("(huber delta r)").unwrap();
    let _ = parse_to_expr("(pseudo_huber delta r)").unwrap();
}

#[test]
fn test_convert_kolmogorov_smirnov() {
    let _ = parse_to_expr("(kolmogorov x)").unwrap();
    let _ = parse_to_expr("(kolmogi p)").unwrap();
    let _ = parse_to_expr("(smirnov n x)").unwrap();
    let _ = parse_to_expr("(smirnovi n p)").unwrap();
}

#[test]
fn test_convert_faddeeva() {
    let _ = parse_to_expr("(wofz z)").unwrap();
    let _ = parse_to_expr("(faddeeva z)").unwrap();
}

#[test]
fn test_convert_diric_tukey() {
    let _ = parse_to_expr("(diric n x)").unwrap();
    let _ = parse_to_expr("(tklmbda lam x)").unwrap();
}

#[test]
fn test_convert_gamma_beta_inverse() {
    let _ = parse_to_expr("(gammaincinv a y)").unwrap();
    let _ = parse_to_expr("(gammainccinv a y)").unwrap();
    let _ = parse_to_expr("(betaincinv a b y)").unwrap();
    let _ = parse_to_expr("(betaincc a b x)").unwrap();
    let _ = parse_to_expr("(betainccinv a b y)").unwrap();
}

#[test]
fn test_convert_high_precision() {
    let _ = parse_to_expr("(cosm1 x)").unwrap();
    let _ = parse_to_expr("(powm1 x y)").unwrap();
    let _ = parse_to_expr("(exp10 x)").unwrap();
    let _ = parse_to_expr("(log1pmx x)").unwrap();
    let _ = parse_to_expr("(loggamma x)").unwrap();
}

#[test]
fn test_convert_degree_trig() {
    let _ = parse_to_expr("(cosdg x)").unwrap();
    let _ = parse_to_expr("(sindg x)").unwrap();
    let _ = parse_to_expr("(tandg x)").unwrap();
    let _ = parse_to_expr("(cotdg x)").unwrap();
    let _ = parse_to_expr("(radian d m s)").unwrap();
}

#[test]
fn test_convert_airy_extensions() {
    let _ = parse_to_expr("(airy x)").unwrap();
    let _ = parse_to_expr("(airye x)").unwrap();
    let _ = parse_to_expr("(aie x)").unwrap();
    let _ = parse_to_expr("(bie x)").unwrap();
    let _ = parse_to_expr("(aip x)").unwrap();
    let _ = parse_to_expr("(bip x)").unwrap();
    let _ = parse_to_expr("(itairy x)").unwrap();
}

#[test]
fn test_convert_exp_int_extensions() {
    let _ = parse_to_expr("(expn n x)").unwrap();
    let _ = parse_to_expr("(exp1 x)").unwrap();
    let _ = parse_to_expr("(shi x)").unwrap();
    let _ = parse_to_expr("(chi x)").unwrap();
}

#[test]
fn test_convert_struve_integrals() {
    let _ = parse_to_expr("(itstruve0 x)").unwrap();
    let _ = parse_to_expr("(it2struve0 x)").unwrap();
    let _ = parse_to_expr("(itmodstruve0 x)").unwrap();
}

#[test]
fn test_convert_ml_statistics() {
    let _ = parse_to_expr("(log_expit x)").unwrap();
    let _ = parse_to_expr("(softplus x)").unwrap();
    let _ = parse_to_expr("(log_ndtr x)").unwrap();
    let _ = parse_to_expr("(softmax x)").unwrap();
    let _ = parse_to_expr("(log_softmax x)").unwrap();
    let _ = parse_to_expr("(logsumexp x)").unwrap();
}

#[test]
fn test_convert_number_theory() {
    let _ = parse_to_expr("(bernoulli n)").unwrap();
    let _ = parse_to_expr("(euler n)").unwrap();
    let _ = parse_to_expr("(binom n k)").unwrap();
}

#[test]
fn test_convert_kelvin_derivatives() {
    let _ = parse_to_expr("(berp x)").unwrap();
    let _ = parse_to_expr("(beip x)").unwrap();
    let _ = parse_to_expr("(kerp x)").unwrap();
    let _ = parse_to_expr("(keip x)").unwrap();
}

#[test]
fn test_convert_bessel_integrals() {
    let _ = parse_to_expr("(besselpoly a lmbda nu)").unwrap();
    let _ = parse_to_expr("(log_wright_bessel a b x)").unwrap();
}

#[test]
fn test_convert_scipy_distributions() {
    let _ = parse_to_expr("(bdtr k n p)").unwrap();
    let _ = parse_to_expr("(bdtrc k n p)").unwrap();
    let _ = parse_to_expr("(chdtr v x)").unwrap();
    let _ = parse_to_expr("(chdtrc v x)").unwrap();
    let _ = parse_to_expr("(fdtr dfn dfd x)").unwrap();
    let _ = parse_to_expr("(fdtrc dfn dfd x)").unwrap();
    let _ = parse_to_expr("(stdtr df t)").unwrap();
    let _ = parse_to_expr("(stdtrc df t)").unwrap();
    let _ = parse_to_expr("(pdtr k m)").unwrap();
    let _ = parse_to_expr("(pdtrc k m)").unwrap();
    let _ = parse_to_expr("(btdtr a b x)").unwrap();
    let _ = parse_to_expr("(gdtr a b x)").unwrap();
    let _ = parse_to_expr("(gdtrc a b x)").unwrap();
}

#[test]
fn test_convert_integral_combinations() {
    let _ = parse_to_expr("(sici x)").unwrap();
    let _ = parse_to_expr("(shichi x)").unwrap();
}

#[test]
fn test_convert_gsl_extensions() {
    let _ = parse_to_expr("(ai_zero s)").unwrap();
    let _ = parse_to_expr("(bi_zero s)").unwrap();
    let _ = parse_to_expr("(bessel_zero_j0 s)").unwrap();
    let _ = parse_to_expr("(bessel_zero_j1 s)").unwrap();
    let _ = parse_to_expr("(bessel_zero_jnu nu s)").unwrap();
    let _ = parse_to_expr("(sph_legendre l m theta)").unwrap();
    let _ = parse_to_expr("(clausen x)").unwrap();
    let _ = parse_to_expr("(debye n x)").unwrap();
    let _ = parse_to_expr("(synchrotron1 x)").unwrap();
    let _ = parse_to_expr("(synchrotron2 x)").unwrap();
    let _ = parse_to_expr("(transport n x)").unwrap();
    let _ = parse_to_expr("(fermi_dirac j x)").unwrap();
}

#[test]
fn test_convert_riemann_siegel() {
    let _ = parse_to_expr("(riemann_siegel_z t)").unwrap();
    let _ = parse_to_expr("(riemann_siegel_theta t)").unwrap();
}

#[test]
fn test_convert_modified_fresnel() {
    let _ = parse_to_expr("(modfresnelp x)").unwrap();
    let _ = parse_to_expr("(modfresnelm x)").unwrap();
}

#[test]
fn test_convert_ellipkm1() {
    let _ = parse_to_expr("(ellipkm1 p)").unwrap();
}
