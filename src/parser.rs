use nom::IResult;
use crate::language::Lang;
use crate::elements::parse_elements;
use nom::character::complete::multispace0;
use nom::sequence::tuple;
use nom::sequence::terminated;
use nom::bytes::complete::tag;
use crate::elements::variable;
use crate::types::type_alias;
use nom::branch::alt;
use crate::types::ltype;
use crate::r#type::Type;
use crate::var::Var;
use nom::combinator::opt;
use crate::types::pascal_case;
use nom::sequence::delimited;
use nom::sequence::preceded;
use crate::elements::tag_exp;
use nom::character::complete::not_line_ending;
use nom::character::complete::line_ending;
use crate::elements::bang_exp;
use nom::multi::many0;
use crate::adt::Adt;

fn pattern_var(s: &str) -> IResult<&str, (Vec<Lang>, Option<String>)> {
    let res = alt((tag_exp, variable))(s);
    match res {
        Ok((s, Lang::Tag(name, val)))
            => {
                if let Lang::Variable(name2, path, perm, mutopa, typ) = *val {
                    Ok((s, 
                        (vec![Lang::Variable(name2.to_string(), path.to_string(), perm, mutopa, typ)],
                        Some(name.to_string()))))
                } else {
                    Ok((s, (vec![], Some(name.to_string()))))
                }
            } ,
        Ok((s, Lang::Variable(name, path, perm, mutopa, typ)))
            => Ok((s, (vec![Lang::Variable(name, path, perm, mutopa, typ)], None))),
        Err(r) => Err(r),
        _ => todo!()
    }
}

fn single_parse(s: &str) -> IResult<&str, Lang> {
    let res = tuple((
        parse_elements,
        terminated(tag(";"), multispace0) 
    ))(s);
    match res {
        Ok((s, (exp, _))) => Ok((s, exp)),
        Err(r) => Err(r)
    }
}

fn equality_operator(s: &str) -> IResult<&str, &str> {
    terminated(alt((tag("="), tag("<-"))), multispace0)(s)
}

fn base_let_exp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
            terminated(tag("let"), multispace0),
            pattern_var,
            opt(preceded(terminated(tag(":"), multispace0), ltype)),
            equality_operator,
            single_parse,
          ))(s);
    match res {
        Ok((s, (_let, (pat_var, None), typ, _eq, Lang::Function(ki, params, ty, body)))) 
            if params.len() > 0 => {
                let newvar = Var::from_language(pat_var[0].clone()).unwrap().set_type(params[0].1.clone()).set_permission(false);
                Ok((s, vec![Lang::Let(newvar, typ.unwrap_or(Type::Empty),
                Box::new(Lang::Function(ki, params, ty, body)))]))
            },
        Ok((s, (_let, (pat_var, None), typ, _eq, body))) => {
                Ok((s, 
                    vec![
                    Lang::Let(
                        Var::from_language(pat_var[0].clone()).unwrap().set_permission(false),
                        typ.clone().unwrap_or(Type::Empty),
                        Box::new(body))]))
                }
        Ok((s, (_let, (pat_var, Some(_)), typ, _eq, body))) => {
            if pat_var.len() == 1 {
                Ok((s, 
                    vec![
                    Lang::Let(
                        Var::from_language(pat_var[0].clone()).unwrap().set_permission(false),
                        typ.clone().unwrap_or(Type::Empty),
                        Box::new(Lang::Dot(Box::new(Lang::Number(0.0)),
                        Box::new(body))))]))

            } else {
                Ok((s,
                pat_var.iter().map(|x| {
                    Lang::Let(Var::from_language(x.clone()).unwrap().set_permission(false), typ.clone().unwrap_or(Type::Empty), Box::new(body.clone()))
                }).collect::<Vec<_>>()
                   ))
            }
        },
        Err(r) => Err(r),
    }
}


fn let_exp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
        opt(terminated(tag("pub"), multispace0)),
        base_let_exp
                    ))(s);
    match res {
        Ok((s, (None, le))) => Ok((s, le)),
        Ok((s, (Some(_pu), le))) => {
            let new_le = le.iter().map(|x| {
                match x {
                    Lang::Let(var, typ, body) 
                        => Lang::Let(var.clone().set_permission(true),
                                    typ.clone(),
                                    body.clone()),
                    lan => lan.clone()
                }
            }).collect();
            Ok((s, new_le))
        },
        Err(r) => Err(r),
    }
}

fn base_mut_exp(s: &str) -> IResult<&str, Lang> {
    let res = tuple((
            terminated(tag("mut"), multispace0),
            variable,
            opt(preceded(terminated(tag(":"), multispace0), ltype)),
            equality_operator,
            single_parse,
          ))(s);
    match res {
        Ok((s, (_met, var, typ, _eq, Lang::Function(ki, params, ty, body)))) 
            if params.len() > 0 => {
                let newvar = Var::from_language(var)
                    .unwrap()
                    .set_type(params[0].1.clone())
                    .set_mutability(true);
                Ok((s, Lang::Let(newvar, typ.unwrap_or(Type::Empty),
                Box::new(Lang::Function(ki, params, ty, body)))))
            },
        Ok((s, (_let, var, typ, _eq, body))) => {
            Ok((s, Lang::Let(
                        Var::from_language(var)
                            .unwrap()
                            .set_mutability(true),
                            typ.unwrap_or(Type::Empty), Box::new(body))))
        },
        Err(r) => Err(r)
    }
}

fn mut_exp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
        opt(terminated(tag("pub"), multispace0)),
        base_mut_exp
                    ))(s);
    match res {
        Ok((s, (None, le))) => Ok((s, vec![le])),
        Ok((s, (Some(_pu), Lang::Let(var, typ, body)))) 
            => Ok((s, vec![Lang::Let(
                    var.clone().set_permission(true),
                    typ.clone(),
                    body.clone())] 
                   )),
        Err(r) => Err(r),
        _ => todo!()
    }
}

fn base_type_exp(s: &str) -> IResult<&str, Lang> {
    let res = tuple((
            terminated(tag("type"), multispace0),
            type_alias,
            terminated(tag("="), multispace0),
            ltype,
            terminated(tag(";"), multispace0) 
          ))(s);
    match res {
        Ok((s, (_ty, Type::Alias(name, params, path), _eq, ty, _))) 
            => Ok((s, Lang::Alias(
                        Var::from_name(&name)
                            .set_type(Type::Params(params.clone()))
                            .add_path(&path),
                        params, ty))),
        Ok((s, (_ty, _, _eq, _ty2, _)))
            => Ok((s, Lang::Empty)),
        Err(r) => Err(r),
    }
}

fn type_exp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
                opt(terminated(tag("pub"), multispace0)),
                base_type_exp
                    ))(s);
    match res {
        Ok((s, (Some(_pu), ali))) => Ok((s, vec![ali])),
        Ok((s, (None, Lang::Alias(var, params, typ)))) 
            => Ok((s, vec![Lang::Alias(
                        var.set_permission(false),
                        params,
                        typ)])),
        Err(r) => Err(r),
        _ => todo!()
    }
}

fn base_opaque_exp(s: &str) -> IResult<&str, Lang> {
    let res = tuple((
            terminated(tag("opaque"), multispace0),
            type_alias,
            terminated(tag("="), multispace0),
            ltype,
            terminated(tag(";"), multispace0) 
          ))(s);
    match res {
        Ok((s, (_ty, Type::Alias(name, params, path), _eq, ty, _))) 
            => Ok((s, Lang::Alias(
                        Var::from_name(&name)
                            .set_type(Type::Params(params.clone()))
                            .add_path(&path)
                            .set_opacity(true),
                        params, ty))),
        Ok((s, (_ty, _, _eq, _ty2, _)))
            => Ok((s, Lang::Empty)),
        Err(r) => Err(r),
    }
}

fn opaque_exp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
                opt(terminated(tag("pub"), multispace0)),
                base_opaque_exp
                    ))(s);
    match res {
        Ok((s, (Some(_pu), Lang::Alias(var, params, typ)))) 
            => Ok((s, vec![Lang::Alias(var.set_opacity(true), params, typ)])),
        Ok((s, (None, Lang::Alias(var, params, typ)))) 
            => Ok((s, vec![Lang::Alias(
                        var.set_permission(false).set_opacity(true),
                        params,
                        typ)])),
        Err(r) => Err(r),
        _ => todo!()
    }
}

pub fn module(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
        terminated(tag("module"), multispace0),
        pascal_case,
        terminated(tag("{"), multispace0),
        parse_exp,
        terminated(tag("}"), multispace0),
        terminated(tag(";"), multispace0)
          ))(s);
    match res {
        Ok((s, (_mod, name, _op, Lang::Sequence(v), _cl, _dv))) => 
            Ok((s, vec![Lang::Module(name, v)])),
        Err(r) => Err(r),
        _ => todo!()
    }
}

pub fn return_exp(s: &str) -> IResult<&str, Lang> {
    let res = terminated(delimited(tag("return "), parse_elements, tag(";")), multispace0)(s);
    match res {
        Ok((s, el)) => Ok((s, Lang::Return(Box::new(el)))),
        Err(r) => Err(r)
    }
}

fn assign(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
            variable,
            alt((
                terminated(tag("="), multispace0),
                terminated(tag("<-"), multispace0))),
            parse_elements,
            terminated(tag(";"), multispace0)
                    ))(s);
    match res {
        Ok((s, (var, _eq, exp, _pv))) => Ok((s, vec![Lang::Assign(Box::new(var), Box::new(exp))])),
        Err(r) => Err(r)
    } 
}

fn comment(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
            tag("#"),
            not_line_ending,
            opt(line_ending),
            multispace0
                    ))(s);
    match res {
        Ok((s, (_hashtag, txt, _, _))) => Ok((s, vec![Lang::Comment(txt.to_string())])),
        Err(r) => Err(r)
    }
}

fn bangs_exp(s: &str) -> IResult<&str,Vec<Lang>> {
    let res = bang_exp(s);
    match res {
        Ok((s, exp)) => Ok((s, vec![exp])),
        Err(r) => Err(r)
    }
}

fn mod_imp(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
            terminated(tag("mod"), multispace0),
            pascal_case,
            terminated(tag(";"), multispace0)
                    ))(s);
    match res {
        Ok((s, (_mod, name, _sc))) => Ok((s, vec![Lang::ModImp(name.to_string())])),
        Err(r) => Err(r)
    }
}

fn import_var(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
                terminated(tag("use"), multispace0),
                variable,
                terminated(tag(";"), multispace0)
                    ))(s);
    match res {
        Ok((s, (_use, Lang::Variable(name, path, perm, mutop, typ), _sc))) => {
            let var1 =  Lang::Variable(name.clone(), path.clone(), perm.clone(), mutop.clone(), typ.clone());
            let var2 =  Lang::Variable(name.clone(), "".to_string(), perm.clone(), mutop.clone(), typ.clone());
            let shortcut = Lang::Let(Var::from_language(var2).unwrap(), Type::Any, Box::new(var1));
            Ok((s, vec![shortcut]))
        }
        Err(r) => Err(r),
        _ => todo!()
    }
}

fn import_type(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
            terminated(tag("use"), multispace0),
            type_alias,
            terminated(tag(";"), multispace0)
                    ))(s);

    match res {
        Ok((s, (_use, alias, _sc))) => Ok((s, vec![Lang::Import(alias)])),
        Err(r) => Err(r)
    }
}

fn tests(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
                tag("Test"),
                delimited(tag("["), base_parse, tag("]"))
                  ))(s);
    match res {
        Ok((s, (_t, body))) => Ok((s, vec![Lang::Test(body)])),
        Err(r) => Err(r)
    }
}

// main
fn base_parse(s: &str) -> IResult<&str, Vec<Lang>> {
    let res = tuple((
        opt(multispace0),
        many0(alt((tests, import_type, import_var, mod_imp, comment, type_exp, mut_exp, opaque_exp, let_exp, module, assign, bangs_exp))),
        opt(alt((return_exp, parse_elements)))
              ))(s);
    match res {
        Ok((s, (_, v, Some(exp)))) => {
            let mut new_v = v.iter().flatten().cloned().collect::<Vec<_>>();
            new_v.push(exp);
            Ok((s, new_v))
        },
        Ok((s, (_, v, None))) => Ok((s, v.iter().flatten().cloned().collect())),
        Err(r) => Err(r)
    }
}

pub fn parse_exp(s: &str) -> IResult<&str, Lang> {
    let res = base_parse(s);
    match res {
        Ok((s, v)) => Ok((s, Lang::Sequence(v.clone()))),
        Err(r) => Err(r)
    }
}

// main
pub fn parse(s: &str) -> IResult<&str, Adt> {
    let res = base_parse(s);
    match res {
        Ok((s, v)) => Ok((s, Adt(v.clone()))),
        Err(r) => Err(r)
    }
}

// main test
#[cfg(test)]
mod tesus {
    use super::*;

    #[test]
    fn test_type_exp1() {
        let res = type_exp("type Num = num;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_type_exp2() {
        let res = type_exp("type Mat<M, N, T> = [M, [N, T]];").unwrap().0;
        assert_eq!(res, "alias(var('Mat'), [M, N, T], [M, [N, T]])");
    }

    #[test]
    fn test_type_exp3() {
        let res = type_exp("type Point = {x: num, y: num};").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_type_exp4() {
        let res = type_exp("type Nn = Un | Deux;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let1() {
        let res = let_exp("let hello: num = 4 ;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let2() {
        let res = let_exp("let a : Num<num> = 3;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let3() {
        let res = let_exp("let a : num = 3;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let4() {
        let res = let_exp("let a : Num<2> = 3;").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let5() {
        let res = let_exp("let ma: Mat<num, num> = [[0, 0], [0, 0]];").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let6() {
        let res = let_exp("let f = fn(a: num, b: bool): num {...};").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_let7() {
        let res = let_exp("let f <- fn(): bool { true };").unwrap().1;
        assert_eq!(res, vec![])
    }

    #[test]
    fn test_suite1() {
        let res = parse("let a = 5; let b = 6; b;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_suite2() {
        let res = parse("type Nu = num;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_suite3() {
        let res = parse("type nu = num; type bo = bool;").unwrap().1;
        assert_eq!(res.0, vec![])
    }
    #[test]
    fn test_suite4() {
        let res = parse("type B = num; let a: Re<num> = record {b: true, t: 10}; a;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_suite5() {
        let res = parse("type C = num; let a: Re<num> = 5; type B<T> = num; b;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_suite6() {
        let res = parse("let z: Mat<2, 2, num> = 8; type Mat<M, N, T> = [M, [N, T]]; let a: Mat<2, 2, num> = [[2, 2], [2, 2]]; a;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_parse_chars() {
        let res = parse("let p1: chars = \"hey\";").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_base_parse1() {
        let res = base_parse("let a = 5;").unwrap().1;
        assert_eq!(res, vec![Lang::Empty]);
    }

    #[test]
    fn test_base_parse2() {
        let res = base_let_exp("let a = 5;").unwrap().1;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_base_parse3() {
        let res = base_mut_exp("mut a = 5;").unwrap().1;
        assert_eq!(res, Lang::Empty)
    }

    #[test]
    fn test_base_parse4() {
        let res = parse("module Person { let nom = 'Jean'; }").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_base_parse5() {
        let res = parse("let a: Combo = 4;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_module1() {
        let res = module("module Add { let a = 5; }").unwrap().1;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_module2() {
        let res = module("module Add { let a = 5; }").unwrap().1;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_module5() {
        let res = parse_exp("module Add { let a = 5; }").unwrap().1;
        assert_eq!(res, Lang::Empty)
    }

    #[test]
    fn test_parse_module() {
        let res = parse("module Add { let a = 5; }").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_option_type1() {
        let res = parse("let a: Option<T> = Some(7);").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_pattern_var1() {
        let res = pattern_var("Some(n)").unwrap().1.0;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_pattern_var2() {
        let res = base_let_exp("let Some(n) = Some(5);").unwrap().1;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_func1() {
        let res = base_let_exp("let pres <- fn(a: {name: chars, age: num}): num { ... };").unwrap().1;
        assert_eq!(res, vec![]);
    }

    #[test]
    fn test_classic_let() {
        let res = parse("let duck = record { wing: Wings, name: 'Daffy'};").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_let_func1() {
        let res = parse("let map <- fn(a: [N, T], f: (T) -> U): [N, U] { ... };").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_let_func2() {
        let res = parse("let append <- fn(a: [#N, T], b: T): [#N+1, T] { ... };").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_assignement1() {
        let res = parse("Mod::a <- 4;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_alias_alias() {
        let res = parse("type Truc = Machin;").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_parse_range1() {
        let res = parse("1:a").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_parse_range2() {
        let res = parse("a:3").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_parse_range3() {
        let res = parse("1:a:3").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_parse_op1() {
        let res = parse("let less_than_3 = fn(n: int): bool { 3 >= n };").unwrap().1;
        assert_eq!(res.0, vec![])
    }

    #[test]
    fn test_ret0() {
        let res = return_exp("return a + 1;").unwrap().1;
        assert_eq!(res, Lang::Empty);
    }

    #[test]
    fn test_base_type_exp() {
        let res = base_type_exp("type Combo = .Truc(int) | .Wow(int);").unwrap().1;
        assert_eq!(res, Lang::Empty);
    }
    
    #[test]
    fn test_type_record() {
        let res = base_type_exp("type Plot = { x: [#N, num], y: [#N, num], t: char };").unwrap().1;
        assert_eq!(res, Lang::Empty);
    }

    #[test]
    fn test_let_seq() {
        let res = parse("let seq <- fn(a: #I, b: #J, c: #K): [#J-#I/#K, int] { ... };").unwrap().1;
        assert_eq!(res.0, vec![]);
    }

    #[test]
    fn test_let_mat() {
        let res = parse("let a: Mat<3, 2> <- [[1, 2], [4, 5], [7, 8]];").unwrap().1;
        assert_eq!(res.0, vec![]);
    }

}
