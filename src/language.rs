#![allow(dead_code)]
use crate::r#type::Type;
use crate::var::Var;
use crate::var::Permission;
use serde::Serialize;
use crate::argument_type::ArgumentType;
use crate::argument_value::ArgumentValue;
use crate::argument_kind::ArgumentKind;
use crate::type_checker;
use crate::Context;
use crate::typing;
use crate::unification;
use crate::help_data::HelpData;
use crate::path::Path;
use crate::function_type::FunctionType;
use crate::type_comparison::reduce_type;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Lang {
    Number(f32, HelpData),
    Integer(i32, HelpData),
    Bool(bool, HelpData),
    Char(String, HelpData),
    And(Box<Lang>, Box<Lang>, HelpData),
    Or(Box<Lang>, Box<Lang>, HelpData),
    Union(Box<Lang>, Box<Lang>, HelpData),
    In(Box<Lang>, Box<Lang>, HelpData),
    Add(Box<Lang>, Box<Lang>, HelpData),
    Eq(Box<Lang>, Box<Lang>, HelpData),
    Eq2(Box<Lang>, Box<Lang>, HelpData),
    NotEq(Box<Lang>, Box<Lang>, HelpData),
    Modu(Box<Lang>, Box<Lang>, HelpData), // modulus
    Modu2(Box<Lang>, Box<Lang>, HelpData), // modulus2
    LesserThan(Box<Lang>, Box<Lang>, HelpData),
    GreaterThan(Box<Lang>, Box<Lang>, HelpData),
    LesserOrEqual(Box<Lang>, Box<Lang>, HelpData),
    GreaterOrEqual(Box<Lang>, Box<Lang>, HelpData),
    Chain(Box<Lang>, Box<Lang>, HelpData),
    Scope(Vec<Lang>, HelpData),
    Function(Vec<ArgumentKind>, Vec<ArgumentType>, Type, Box<Lang>, HelpData),
    Module(String, Vec<Lang>, HelpData), // module name { lines }
    ModuleDecl(String, HelpData), // to create an env
    Variable(String, Path, Permission, bool, Type, HelpData),
    FunctionApp(Box<Lang>, Vec<Lang>, HelpData),
    ArrayIndexing(Box<Lang>, i32, HelpData),
    Let(Var, Type, Box<Lang>, HelpData),
    Array(Vec<Lang>, HelpData),
    Record(Vec<ArgumentValue>, HelpData),
    Alias(Var, Vec<Type>, Type, HelpData),
    Tag(String, Box<Lang>, HelpData),
    If(Box::<Lang>, Box<Lang>, Box<Lang>, HelpData),
    Match(Box<Lang>, Vec<(Box<Lang>, Box<Lang>)>, HelpData),
    Tuple(Vec<Lang>, HelpData),
    Sequence(Vec<Lang>, HelpData),
    Assign(Box<Lang>, Box<Lang>, HelpData),
    Comment(String, HelpData),
    Range(i32, i32, i32, HelpData),
    ModImp(String, HelpData), // mod name;
    Import(Type, HelpData), // type alias
    Header(Box<Lang>, HelpData),
    GenFunc(String, String, HelpData), //body, name, helpdata
    Test(Vec<Lang>, HelpData),
    Return(Box<Lang>, HelpData),
    VecBloc(String, HelpData),
    Lambda(Box<Lang>, HelpData),
    Library(String, HelpData),
    Exp(String, HelpData),
    Any(HelpData),
    Signature(Var, Type, HelpData),
    ForLoop(Var, Box<Lang>, Box<Lang>, HelpData), // variable, iterator, body
    RFunction(Vec<Lang>, String, HelpData), // variable, iterator, body
    Empty(HelpData)
}

impl From<Var> for Lang {
   fn from(val: Var) -> Self {
       Lang::Variable(val.0, val.1.into(), val.2, val.3, val.4, val.5)
   } 
}

fn my_to_str<T: ToString>(v: &[T]) -> String {
    let res = v.iter()
        .map(|x| x.to_string())
        .reduce(|acc, x| format!("{}, {}", acc, x))
        .unwrap_or("".to_string());
    format!("[{}]", res)
}

pub fn build_generic_function(s: &str) -> String {
    format!("{} <- function(x, ...) {{\n\tUseMethod('{}')\n}}\n", s, s)
}

fn condition_to_if(var: &Lang, condition: &Lang) -> (String, Lang) {
    match (var, condition) {
        (Lang::Variable(name, _, _ , _, _, _), Lang::Tag(tag_name, body, _)) => {
            (format!("{}[[1]] == '{}'", name, tag_name), (**body).clone())
        },
        _ => panic!("The element you put next to 'match' isn't a variable or your left part of your branches aren't true tags")
    }
}

fn to_if_statement(var: Lang, branches: &[(Box<Lang>, Box<Lang>)], context: &Context) -> String {
    branches.iter()
        .map(|(condition, body)| (condition_to_if(&var, condition), body))
        .map(|((cond, sub_var), body)| (cond, body.lang_substitution(&sub_var, &var, context)))
        .enumerate()
        .map(|(id, (cond, body))| if id == 0 {
            format!("if ({}) {{ \n {} \n }}", cond, body)
        } else {
            format!("else if ({}) {{ \n {} \n }}", cond, body)
        }).collect::<Vec<_>>().join(" ")
}

//main
impl Lang {
    fn set_type(&self, typ: &Type) -> Lang {
        match self {
            Lang::Variable(name, path, perm, spec, _, h) 
                => Lang::Variable(name.clone(), path.clone(), perm.clone(), spec.clone(), typ.clone(), h.clone()),
            _ => self.clone()
        }
    }
    pub fn shape(&self) -> Vec<usize> {
        match self {
            Lang::Array(vec, _) => {
                let dimensions = vec.len(); // Taille actuelle de ce niveau
                if let Some(first) = vec.get(0) {
                    if let Lang::Array(_, _) = first {
                        // Descend récursivement dans la première sous-structure
                        let mut sub_shape = first.shape();
                        sub_shape.insert(0, dimensions);
                        sub_shape
                    } else {
                        // Si ce niveau contient des valeurs uniquement
                        vec![dimensions]
                    }
                } else {
                    vec![0] // Array vide
                }
            }
            _ => vec![], // Retourne une forme vide si ce n'est pas un tableau
        }
    }

    pub fn to_r(&self, cont: &Context) -> (String, Context) {
        let result = match self {
            Lang::Bool(b, _) => 
                (format!("{}", b.to_string().to_uppercase()), cont.clone()),
            Lang::In(b1, b2, _) => {
                let (b1_str, cont1) = b1.to_r(cont);
                let (b2_str, cont2) = b2.to_r(&cont1);
                (format!("{} %in% {}", b2_str, b1_str), cont2)
            },
            Lang::And(b1, b2, _) => {
                let (b1_str, cont1) = b1.to_r(cont);
                let (b2_str, cont2) = b2.to_r(&cont1);
                (format!("{} & {}", b1_str, b2_str), cont2)
            },
            Lang::Or(b1, b2, _) => {
                let (b1_str, cont1) = b1.to_r(cont);
                let (b2_str, cont2) = b2.to_r(&cont1);
                (format!("{} | {}", b1_str, b2_str), cont2)
            },
            Lang::Modu(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} % {}", e2_str, e1_str), cont2)
            },
            Lang::Modu2(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} %% {}", e2_str, e1_str), cont2)
            },
            Lang::Number(n, _) => 
                (format!("{}", n), cont.clone()),
            Lang::Add(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("add({}, {})", e1_str, e2_str), cont2)
            },
            Lang::Eq(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} == {}", e2_str, e1_str), cont2)
            },
            Lang::NotEq(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} != {}", e2_str, e1_str), cont2)
            },
            Lang::LesserThan(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} < {}", e2_str, e1_str), cont2)
            },
            Lang::GreaterThan(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} > {}", e2_str, e1_str), cont2)
            },
            Lang::LesserOrEqual(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} <= {}", e2_str, e1_str), cont2)
            },
            Lang::GreaterOrEqual(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_r(cont);
                let (e2_str, cont2) = e2.to_r(&cont1);
                (format!("{} >= {}", e2_str, e1_str), cont2)
            },
            Lang::Chain(e1, e2, _) => {
                match *e1.clone() {
                    Lang::Variable(_, _, _, _, _, _) => {
                        let (e1_str, cont1) = e1.to_r(cont);
                        let (e2_str, cont2) = e2.to_r(&cont1);
                        (format!("{}[[{}]]", e2_str, e1_str), cont2)
                    },
                    Lang::Record(fields, _) => {
                        let (e2_str, cont2) = e2.to_r(cont);
                        let at = fields[0].clone();
                        let res = format!("within({}, {{ {} <- {} }})", 
                                e2_str, at.get_argument(), at.get_value().to_r(&cont).0);
                        (res, cont2)
                    }
                    Lang::FunctionApp(_, _, _) => {
                        let (e1_str, _cont1) = e1.to_r(cont);
                        let (e2_str, cont2) = e2.to_r(&cont);
                        (format!("{} |> {}", e2_str, e1_str), cont2)
                    }
                    _ => {
                        let (e1_str, cont1) = e1.to_r(cont);
                        let (e2_str, cont2) = e2.to_r(&cont1);
                        (format!("{}[[{}]]", e2_str, e1_str), cont2)
                    }
                }
            },
            Lang::Scope(exps, _) => {
                let mut current_cont = cont.clone();
                let mut results = Vec::new();
                
                for exp in exps {
                    let (exp_str, new_cont) = exp.to_r(&current_cont);
                    results.push(exp_str);
                    current_cont = new_cont;
                }
                
                (results.join("\n"), current_cont)
            },
            Lang::Function(_args_kind, args, _typ, body, _h) => {
                let sub_cont = cont.add_arg_types(args);
                let (body_str, new_cont) = body.to_r(&sub_cont);
                let fn_type = typing(&sub_cont, self).0;
                let class = cont.get_class(&fn_type);
                let classes = cont.get_classes(&fn_type)
                    .unwrap_or("''".to_string());
                (format!("(function({}) {{\n {} \n}}) |>\n\t struct(c('{}', {}))", 
                        args.iter().map(|x| x.to_r()).collect::<Vec<_>>().join(", "),
                        body_str, class, classes), 
                new_cont)
            },
            Lang::Variable(v, path, _perm, _muta, _ty, _) => {
                let name = if v.contains("__") {
                    v.replace("__", ".")
                } else {
                    match _ty {
                        Type::Empty(_) | Type::Any(_) => v.clone(),
                        _ => v.clone() + "." + &cont.get_class(_ty)
                    }
                };
                ((path.clone().to_r() + &name).to_string(), cont.clone())
            }
            Lang::FunctionApp(exp, vals, _) => {
                let (exp_str, cont1) = exp.to_r(cont);
                let (unification_map, _cont2) = cont1.pop_unifications();
                let fn_type = typing(cont, exp).0;
                let new_fn_typ = unification::type_substitution(&fn_type, &unification_map.unwrap_or(vec![]));

                let new_vals = match new_fn_typ {
                    Type::Function(_, args, _, _) => {
                        let new_args = args.into_iter()
                            .map(|arg| reduce_type(&cont1, &arg))
                            .collect::<Vec<_>>();
                        vals.into_iter().zip(new_args.iter())
                            .map(|(val, arg)| {
                                match arg {
                                    Type::Function(_, args2, _, _) 
                                        if args2.len() > 0
                                        => val.set_type(&args2[0]),
                                    _ => val.clone()
                                }
                            }).collect::<Vec<_>>()
                        },
                    _ => vals.clone()
                };
                
                let mut current_cont = cont1;
                let mut val_strs = Vec::new();

                for val in new_vals {
                    let (val_str, new_cont) = val.to_r(&current_cont);
                    val_strs.push(val_str);
                    current_cont = new_cont;
                }
                
                let args = val_strs.join(", ");
                
                match *exp.clone() {
                    Lang::Variable(name, path, _perm, _spec, _typ, _) => {
                        let new_name = name.replace("__", ".");
                        if path != Path::default() {
                            (format!("eval(quote({}({})), envir = {})",
                                new_name, args, path.get_value()), current_cont)
                        } else {
                            (format!("{}({})", name.replace("__", "."), args), current_cont)
                        }
                    },
                    _ => (format!("{}({})", exp_str, args), current_cont)
                }
            },
            Lang::ArrayIndexing(exp, val, _) => {
                let (exp_str, new_cont) = exp.to_r(cont);
                (format!("{}[{}]", exp_str, val), new_cont)
            },
            Lang::GenFunc(func, _, _) => 
                (func.to_string(), cont.clone()),
            Lang::Let(var, ttype, body, _) => {
                let (body_str, new_cont) = body.to_r(cont);
                let new_name = var.clone().to_r();
                
                let (r_code, _new_name2) = match (**body).clone() {
                    Lang::Function(_, _, _, _, _) => {
                        let related_type = var.get_type();
                        let class = match related_type {
                            Type::Empty(_) => "Empty".to_string(),
                            Type::Any(_) => "Generic_".to_string(),
                            _ => cont.get_class(&reduce_type(cont, &related_type))
                        };
                        if class.len() > 7 && &class[0..7] == "Generic" {
                            (format!("{}.default <- {}", new_name, body_str), new_name)
                        } else if class == "Empty" {
                            (format!("{} <- {}", new_name, body_str), new_name)
                        } else {
                            let new_name2 = format!("{}.{}", new_name, class);
                            (format!("{} <- {}", new_name2, body_str), new_name2)
                        }
                    }
                    _ => (format!("{} <- {}", new_name, body_str), new_name)
                };

                let classes_res = new_cont.get_classes(ttype);

                match classes_res {
                    Some(classes) =>
                        (format!("{} |> \n\tlet_type({})\n", r_code, classes),
                        new_cont),
                    None => (r_code + "\n", new_cont)
                }
            },
            Lang::Array(v, _h) => {
                let str_linearized_array = &self.linearize_array()
                    .iter()
                    .map(|lang| lang.to_r(&cont).0)
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let dim = typing(&cont, &self).0;
                let shape = dim.get_shape().unwrap();
                let classes = cont.get_classes(&dim).unwrap_or("''".to_string());

                let vector = if str_linearized_array == "" {
                   "logical(0)".to_string()
                } else {
                    format!("c({})", str_linearized_array)
                };

                let array = if shape.contains("dim(===)") {
                    format!("array({}, dim = c({}))", vector,
                                shape.replace("===", &v[0].to_r(&cont).0))
                } else {
                    format!("array({}, dim = c({}))", vector, shape)
                };

                (format!("{} |> \n\tstruct({})", array, classes)
                 ,cont.to_owned())
            },
            Lang::Record(args, _) => {
                let mut current_cont = cont.clone();
                let mut arg_strs = Vec::new();
                
                for arg in args {
                    let (arg_str, new_cont) = (arg.to_r(&current_cont), cont.clone());
                    arg_strs.push(arg_str);
                    current_cont = new_cont;
                }
                
                let body = arg_strs.join(", ");
                let typ = type_checker::typing(cont, self).0;
                let class = cont.get_class(&typ);
                match cont.get_classes(&typ) {
                    Some(res) => (format!("struct(list({}), c('list', 'Record', '{}', {}))", body, class, res), current_cont),
                    _ => (format!("struct(list({}), c('list', 'Record', '{}'))", body, class), current_cont)
                }
            },
            Lang::Char(s, _) => 
                ("'".to_string() + s + "'", cont.clone()),
            Lang::If(cond, exp, els, _) if els == &Box::new(Lang::Empty(HelpData::default())) => {
                let (cond_str, cont1) = cond.to_r(cont);
                let (exp_str, cont2) = exp.to_r(&cont1);
                
                (format!("if({}) {{\n {} \n}}", cond_str, exp_str), cont2)
            },
            Lang::If(cond, exp, els, _) => {
                let (cond_str, cont1) = cond.to_r(cont);
                let (exp_str, cont2) = exp.to_r(&cont1);
                let (els_str, cont3) = els.to_r(&cont2);
                
                (format!("if ({}) {{\n {} \n}} else {}", cond_str, exp_str, els_str), cont3)
            },
            Lang::Tuple(vals, _) => {
                let mut current_cont = cont.clone();
                let mut val_entries = Vec::new();
                
                //for (i, val) in vals.iter().enumerate() {
                    //let (val_str, new_cont) = val.to_r(&current_cont);
                    //val_entries.push(format!("'{}' = {}", i.to_string(), val_str));
                    //current_cont = new_cont;
                //}
                
                for val in vals.iter() {
                    let (val_str, new_cont) = val.to_r(&current_cont);
                    val_entries.push(format!("{}", val_str));
                    current_cont = new_cont;
                }
                
                (format!("struct(list({}), 'Tuple')", val_entries.join(", ")), current_cont)
            },
            Lang::Assign(var, exp, _) => {
                let (var_str, cont1) = var.to_r(cont);
                let (exp_str, cont2) = exp.to_r(&cont1);
                
                (format!("{} <- {}", var_str, exp_str), cont2)
            },
            Lang::Comment(txt, _) => 
                ("# ".to_string() + txt, cont.clone()),
            Lang::Range(i1, i2, i0, _) => 
                (format!("array(seq({},{},{}), dim = c({}))", i1, i2, i0, i2-i1/i0), cont.clone()),
            Lang::Integer(i, _) => 
                (format!("{}L", i), cont.clone()),
            Lang::Tag(s, t, _) => {
                let (t_str, new_cont) = t.to_r(cont);
                let typ = type_checker::typing(cont, self).0;
                let class = cont.get_class(&typ);
                
                match cont.get_classes(&typ) {
                    Some(res) => 
                        (format!("struct(list('{}', {}), c('Tag', '{}', {}))", s, t_str, class, res), new_cont),
                    _ => (format!("struct(list('{}', {}), c('Tag', '{}'))", s, t_str, class), new_cont)
                }
            },
            Lang::Empty(_) => 
                ("NA".to_string(), cont.clone()),
            Lang::ModuleDecl(name, _) => 
                (format!("{} <- new.env()", name), cont.clone()),
            Lang::Sequence(exps, _) => {
                let mut current_cont = cont.clone();
                let mut results = Vec::new();
                
                for exp in exps {
                    let (exp_str, new_cont) = exp.to_r(&current_cont);
                    results.push(exp_str);
                    current_cont = new_cont;
                }
                
                (results.join("\n"), current_cont)
            },
            Lang::Return(exp, _) => {
                let (exp_str, new_cont) = exp.to_r(cont);
                (format!("return ({})", exp_str), new_cont)
            },
            Lang::Lambda(bloc, _) 
                => (format!("function(x) {{ {} }}", bloc.to_r(cont).0), cont.clone()),
            Lang::VecBloc(bloc, _) => (bloc.to_string(), cont.clone()),
            Lang::Library(name, _) => (format!("library({})", name), cont.clone()),
            Lang::Match(var, branches, _) => (to_if_statement((**var).clone(), branches, cont), cont.clone()),
            Lang::Exp(exp, _) => (exp.clone(), cont.clone()),
            Lang::ForLoop(var, iterator, body, _) => {
                let res = format!("for ({} in {}) {{\n {} \n}}", var.clone().to_r(), iterator.to_r(cont).0, body.to_r(cont).0);
                (res, cont.clone())
            },
            Lang::RFunction(vars, body, _) => {
                let args = vars.iter()
                    .map(|x| x.to_r(cont).0)
                    .collect::<Vec<_>>()
                    .join(", ");
                (format!("function ({}) {{\n {} \n}}", args, body), cont.clone())
            }
            Lang::Eq2(right, left, _) => {
                let res = match &**left {
                    Lang::Tag(n, _, _) => n.to_string(),
                    Lang::Variable(n, _, _, _, _, _) => n.to_string(),
                    _ => format!("{}", left) 
                };
                (format!("{} = {}", res, right.to_r(cont).0), cont.clone())
            }
            Lang::Signature(_, _, _) => {
                ("".to_string(), cont.clone())
            }
            Lang::Alias(_, _, _, _) => ("".to_string(), cont.clone()),
            _ =>  {
                println!("This language structure won't transpile: {:?}", self);
                ("".to_string(), cont.clone())
            }
        };
        
        result
    }

    pub fn to_typescript(&self, cont: &Context) -> (String, Context) {
        match self {
            Lang::Bool(b, _) => (format!("{}", b), cont.clone()),
            Lang::Integer(i, _) => (format!("{}", i), cont.clone()),
            Lang::Number(n, _) => (format!("{}", n), cont.clone()),
            Lang::Char(c, _) => (format!("\"{}\"", c), cont.clone()),
            Lang::Empty(_) => ("null".to_string(), cont.clone()),
            Lang::Array(v, _) => {
                let mut current_cont = cont.clone();
                let mut val_strs = Vec::new();
                
                for val in v {
                    let (val_str, new_cont) = val.to_typescript(&current_cont);
                    val_strs.push(val_str);
                    current_cont = new_cont;
                }
                
                (format!("[{}]", val_strs.join(", ")), current_cont)
            },
            Lang::Record(v, _) => {
                let mut current_cont = cont.clone();
                let mut arg_strs = Vec::new();
                
                for arg_val in v {
                    let (val_str, new_cont) = arg_val.get_value().to_typescript(&current_cont);
                    arg_strs.push(format!("{}: {}", arg_val.get_argument(), val_str));
                    current_cont = new_cont;
                }
                
                (format!("{{ {} }}", arg_strs.join(", ")), current_cont)
            },
            Lang::Tuple(vals, _) => {
                let mut current_cont = cont.clone();
                let mut val_entries = Vec::new();
                
                for (i, val) in vals.iter().enumerate() {
                    let (val_str, new_cont) = val.to_typescript(&current_cont);
                    val_entries.push(format!("'{}': {}", i.to_string(), val_str));
                    current_cont = new_cont;
                }
                
                (format!("{{ {} }}", val_entries.join(", ")), current_cont)
            },
            Lang::Tag(s, t, _) => {
                let (t_str, new_cont) = t.to_typescript(cont);
                
                (format!("{{ _type: '{}', _body: {} }}", s, t_str), new_cont)
            },
            Lang::Variable(v, path, _perm, _muta, _ty, _h) => 
                ((path.to_owned() + v.to_owned().into()).to_string(), cont.clone()),
            Lang::Let(var, _typ, body, _h) => {
                if var.get_name() == "main" {
                    match *body.clone() {
                        Lang::Function(_kinds, _params, _ret, body2, _) => {
                            let (body_str, new_cont) = body2.to_typescript(cont);
                            (format!("export function main(): void {{\n{}\n}}", body_str), new_cont)
                        },
                        _ => ("".to_string(), cont.clone()) // todo!()
                    }
                } else {
                   match *body.clone() {
                       Lang::Function(_kinds, params, ret, body2, _) => {
                           let first = params.iter().nth(0).unwrap().get_type();
                           let class = cont.get_class(&first);
                           let res = params.iter()
                            .map(|at| format!("{}: {}",
                                              at.get_argument(),
                                              at.get_type().to_typescript()))
                            .collect::<Vec<_>>();
                            
                            let (body_str, new_cont) = body2.to_typescript(cont);
                            (format!("function {}_{}({}): {} {{\n{}\n}}",
                                class,
                                var.get_name(), 
                                res.join(", "),
                                ret.to_typescript(), 
                                body_str), 
                            new_cont)
                       },
                       _ => {
                           let (body_str, new_cont) = body.to_typescript(cont);
                           (format!("let {} = {}", var.get_name(), body_str), new_cont)
                       }
                   } 
                }
            },
            Lang::FunctionApp(var, params, _) => {
                let first = params.iter().nth(0).unwrap();
                let typ = typing(cont, first).0;
                
                let (_var_str, cont1) = var.to_typescript(cont);
                
                let mut current_cont = cont1;
                let mut param_strs = Vec::new();
                
                for param in params {
                    let (param_str, new_cont) = param.to_typescript(&current_cont);
                    param_strs.push(param_str);
                    current_cont = new_cont;
                }
                
                let name = var.get_name();
                let result = match &name[..] {
                    "parseInt" | "parseFloat" | "map"
                        => format!("{}({})", var.get_name(), param_strs.join(", ")),
                    _n if (name.len() > 6) && (&name[0..6] == "math__") => {
                            format!("Math.{}({})", name[6..].to_string(), param_strs.join(", "))
                        }
                    _n if name.contains("__") =>
                        format!("{}({})", name.replace("__", "."), param_strs.join(", ")),
                    _ => format!("{}_{}({})", cont.get_class(&typ), var.get_name(), param_strs.join(", "))
                };
                
                (result, current_cont)
            },
            Lang::Function(_kinds, args, _ret_typ, body, _) => {
                let params = args.iter()
                    .map(|arg_typ| arg_typ.get_argument_str())
                    .collect::<Vec<_>>().join(", ");
                let cont2 = args.iter()
                    .map(|arg_typ| (arg_typ.get_argument(), arg_typ.get_type()))
                    .map(|(arg, typ)| (Var::from_name(&(arg.get_label())), typ))
                    .fold(cont.clone(), |ctx, (arg, typ)| ctx.clone().push_var_type(arg, typ, &ctx));
                (format!("({}) => {{ {} }}", 
                         params, body.to_typescript(&cont2).0), cont.clone())
            }
            Lang::Scope(langs, _) => {
                let mut current_cont = cont.clone();
                let mut result_strs = Vec::new();
                
                for lang in langs {
                    let (lang_str, new_cont) = lang.to_typescript(&current_cont);
                    result_strs.push(lang_str);
                    current_cont = new_cont;
                }
                
                (result_strs.join("\n"), current_cont)
            },
            Lang::Sequence(exps, _) => {
                let mut current_cont = cont.clone();
                let mut result_strs = Vec::new();
                
                for exp in exps {
                    let (exp_str, new_cont) = exp.to_typescript(&current_cont);
                    result_strs.push(exp_str);
                    current_cont = new_cont;
                }
                
                (result_strs.join("\n\n"), current_cont)
            },
            Lang::Return(exp, _) => {
                let (exp_str, new_cont) = exp.to_typescript(cont);
                (format!("return {};", exp_str), new_cont)
            },
            Lang::Chain(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_typescript(cont);
                let (e2_str, cont2) = e2.to_typescript(&cont1);
                
                match *e1.clone() {
                    Lang::Variable(_, _, _, _, _, _) => 
                        (format!("{}.{}", e2_str, e1_str), cont2),
                    _ => (format!("{} |> {}", e2_str, e1_str), cont2),
                }
            },
            Lang::Alias(var, args, typ, _h) => {
                if args.len() > 0 {
                    let res = args.iter()
                        .map(|typ| typ.to_typescript())
                        .collect::<Vec<_>>()
                        .join(", ");
                    (format!("type {}<{}> = {};", var.get_name(), res, typ.to_typescript()), cont.clone())
                } else {
                    (format!("type {} = {};", var.get_name(), typ.to_typescript()), cont.clone())
                }
            },
            _ => ("".to_string(), cont.clone())
        }
    }

    pub fn to_assemblyscript(&self, cont: &Context) -> (String, Context) {
        match self {
            Lang::Bool(b, _) => (format!("{}", b), cont.clone()),
            Lang::Integer(i, _) => (format!("{}", i), cont.clone()),
            Lang::Number(n, _) => (format!("{}", n), cont.clone()),
            Lang::Char(c, _) => (format!("\"{}\"", c), cont.clone()),
            Lang::Empty(_) => ("null".to_string(), cont.clone()),
            Lang::Array(v, _) => {
                let mut current_cont = cont.clone();
                let mut val_strs = Vec::new();
                
                for val in v {
                    let (val_str, new_cont) = val.to_assemblyscript(&current_cont);
                    val_strs.push(val_str);
                    current_cont = new_cont;
                }
                
                (format!("[{}]", val_strs.join(", ")), current_cont)
            },
            Lang::Record(v, _) => {
                let mut current_cont = cont.clone();
                let mut arg_strs = Vec::new();
                
                for arg_val in v {
                    let (val_str, new_cont) = arg_val.get_value().to_assemblyscript(&current_cont);
                    arg_strs.push(format!("{}: {}", arg_val.get_argument(), val_str));
                    current_cont = new_cont;
                }
                
                (format!("{{ {} }}", arg_strs.join(", ")), current_cont)
            },
            Lang::Tuple(vals, _) => {
                let mut current_cont = cont.clone();
                let mut val_entries = Vec::new();
                
                for (i, val) in vals.iter().enumerate() {
                    let (val_str, new_cont) = val.to_assemblyscript(&current_cont);
                    val_entries.push(format!("'{}': {}", i.to_string(), val_str));
                    current_cont = new_cont;
                }
                
                (format!("{{ {} }}", val_entries.join(", ")), current_cont)
            },
            Lang::Tag(s, t, _) => {
                let (t_str, new_cont) = t.to_assemblyscript(cont);
                
                (format!("{{ _type: '{}', _body: {} }}", s, t_str), new_cont)
            },
            Lang::Variable(v, path, _perm, _muta, _ty, _) => 
                ((path.to_owned() + v.to_owned().into()).to_string(), cont.clone()),
            Lang::Let(var, _typ, body, _) => {
                if var.get_name() == "main" {
                    match *body.clone() {
                        Lang::Function(_kinds, _params, _ret, body2, _h) => {
                            let (body_str, new_cont) = body2.to_assemblyscript(cont);
                            (format!("export function main(): void {{\n{}\n}}", body_str), new_cont)
                        },
                        _ => ("".to_string(), cont.clone()) // todo!()
                    }
                } else {
                   match *body.clone() {
                       Lang::Function(kinds, params, ret, body2, _h) => {
                           let first = params.iter().nth(0).unwrap().get_type();
                           let class = cont.get_class(&first);
                           let res = params.iter()
                            .map(|at| format!("{}: {}",
                                              at.get_argument(),
                                              at.get_type().to_assemblyscript()))
                            .collect::<Vec<_>>();
                            
                            let (body_str, new_cont) = body2.to_assemblyscript(cont);
                            if kinds.len() == 0 {
                                (format!("function {}_{}({}): {} {{\n{}\n}}",
                                    class,
                                    var.get_name(), 
                                    res.join(", "),
                                    ret.to_assemblyscript(), 
                                    body_str), 
                                new_cont)
                            } else {
                                let generics = kinds.iter()
                                    .map(|arg_kind| arg_kind.get_argument())
                                    .map(|kind| kind.to_assemblyscript())
                                    .collect::<Vec<_>>().join(", ");
                                (format!("function {}_{}<{}>({}): {} {{\n{}\n}}",
                                    class,
                                    var.get_name(), 
                                    generics,
                                    res.join(", "),
                                    ret.to_assemblyscript(), 
                                    body_str), 
                                new_cont)
                            }
                       },
                       _ => {
                           let (body_str, new_cont) = body.to_assemblyscript(cont);
                           (format!("let {} = {}", var.get_name(), body_str), new_cont)
                       }
                   } 
                }
            },
            Lang::FunctionApp(var, params, _) => {
                let first = params.iter().nth(0).unwrap();
                let typ = typing(cont, first).0;
                
                let (_var_str, cont1) = var.to_assemblyscript(cont);
                
                let mut current_cont = cont1;
                let mut param_strs = Vec::new();
                
                for param in params {
                    let (param_str, new_cont) = param.to_assemblyscript(&current_cont);
                    param_strs.push(param_str);
                    current_cont = new_cont;
                }
                
                let name = var.get_name();
                let result = match &name[..] {
                    "parseInt" | "parseFloat" | "map"
                        => format!("{}({})", var.get_name(), param_strs.join(", ")),
                    _n if (name.len() > 6) && (&name[0..6] == "math__") => {
                            format!("Math.{}({})", name[6..].to_string(), param_strs.join(", "))
                        }
                    _n if name.contains("__") =>
                        format!("{}({})", name.replace("__", "."), param_strs.join(", ")),
                    _ => format!("{}_{}({})", cont.get_class(&typ), var.get_name(), param_strs.join(", "))
                };
                
                (result, current_cont)
            },
            Lang::Function(_kinds, args, _ret_typ, body, _h) => {
                let params = args.iter()
                    .map(|arg_typ| arg_typ.get_argument_str())
                    .collect::<Vec<_>>().join(", ");
                let cont2 = args.iter()
                    .map(|arg_typ| (arg_typ.get_argument_str(), arg_typ.get_type()))
                    .map(|(arg, typ)| (Var::from_name(&arg), typ))
                    .fold(cont.clone(), |ctx, (arg, typ)| ctx.clone().push_var_type(arg, typ, &ctx));
                (format!("({}) => {{ {} }}", 
                         params, body.to_assemblyscript(&cont2).0), cont.clone())
            }
            Lang::Scope(langs, _) => {
                let mut current_cont = cont.clone();
                let mut result_strs = Vec::new();
                
                for lang in langs {
                    let (lang_str, new_cont) = lang.to_assemblyscript(&current_cont);
                    result_strs.push(lang_str);
                    current_cont = new_cont;
                }
                
                (result_strs.join("\n"), current_cont)
            },
            Lang::Sequence(exps, _) => {
                let mut current_cont = cont.clone();
                let mut result_strs = Vec::new();
                
                for exp in exps {
                    let (exp_str, new_cont) = exp.to_assemblyscript(&current_cont);
                    result_strs.push(exp_str);
                    current_cont = new_cont;
                }
                
                (result_strs.join("\n\n"), current_cont)
            },
            Lang::Return(exp, _) => {
                let (exp_str, new_cont) = exp.to_assemblyscript(cont);
                (format!("return {};", exp_str), new_cont)
            },
            Lang::Chain(e1, e2, _) => {
                let (e1_str, cont1) = e1.to_assemblyscript(cont);
                let (e2_str, cont2) = e2.to_assemblyscript(&cont1);
                
                match *e1.clone() {
                    Lang::Variable(_, _, _, _, _, _) => 
                        (format!("{}.{}", e2_str, e1_str), cont2),
                    _ => (format!("{} |> {}", e2_str, e1_str), cont2),
                }
            },
            Lang::Alias(var, args, typ, _h) => {
                if args.len() > 0 {
                    let res = args.iter()
                        .map(|typ| typ.to_assemblyscript())
                        .collect::<Vec<_>>()
                        .join(", ");
                    (format!("type {}<{}> = {};", var.get_name(), res, typ.to_assemblyscript()), cont.clone())
                } else {
                    match typ {
                        Type::Function(kinds, args, ret_typ, h) => {
                            let res = kinds.iter()
                                .map(|arg_kind| arg_kind.get_argument())
                                .map(|kind| kind.to_assemblyscript())
                                .collect::<Vec<_>>().join(", ");
                            let new_fn = Type::Function(vec![], args.clone(), ret_typ.clone(), h.clone());
                            let res = if res == "" { res } else { format!("<{}>", res)};
                           (format!("type {}{} = {};", 
                                    var.get_name(),
                                    res,
                                    new_fn.to_assemblyscript()),
                            cont.clone()) 
                        },
                        _ => (format!("type {} = {};", var.get_name(), typ.to_assemblyscript()), cont.clone())
                    }
                }
            },
            _ => ("".to_string(), cont.clone())
        }
    }

    pub fn get_number(&self) -> i32 {
        if let Lang::Integer(number, _) = self {
            number.clone()
        } else { 0 }
    }

    pub fn get_name(&self) -> String {
        if let Lang::Variable(name, _, _, _, _, _) = self {
            name.to_string()
        } else { "".to_string() }
    }

    pub fn is_undefined(&self) -> bool {
        if let Lang::Function(_, _, _, body, _h) = self.clone() {
            if let Lang::Scope(v, _) = *body.clone() {
                   let ele = v.first().unwrap();
                   if let Lang::Empty(_) = ele {true} else {false}
            } else {false}
        } else {false}
    }

    pub fn is_function(&self) -> bool {
        match self {
            Lang::Function(_, _, _, _, _) => true,
            Lang::RFunction(_, _, _) => true,
            _ => false
        }
    }

    pub fn infer_var_name(&self, args: &Vec<Lang>, context: &Context) -> Var {
        if args.len() > 0 {
                        let first = typing(context, &args.iter().nth(0).unwrap().clone()).0;
                        Var::from_language(self.clone())
                            .unwrap().set_type(first)
                    } else {
                        Var::from_language(self.clone()).unwrap()
            }
    }

    pub fn get_related_function(self, args: &Vec<Lang>, context: &Context) 
        -> Option<FunctionType> {
        let var_name = self.infer_var_name(args, context);
        let fn_ty = typing(context, &var_name.to_language()).0;
        fn_ty.to_function_type()
    }

    pub fn lang_substitution(&self, sub_var: &Lang, var: &Lang, context: &Context) -> String {
        if let Lang::Variable(name, _, _, _, _, _) = var {
            let res = match self {
                Lang::Variable(_, _, _, _, _, h) if self == sub_var 
                    => Lang::Exp(format!("{}[[2]]", name.to_string()), h.clone()),
                lang => lang.clone()
            };
            res.to_r(context).0
        } else { panic!("var is not a variable") }
    }

    pub fn get_help_data(&self) -> HelpData {
        match self {
            Lang::Number(_, h) => h,
            Lang::Integer(_, h) => h,
            Lang::Char(_, h) => h,
            Lang::Bool(_, h) => h,
            Lang::And(_, _, h) => h,
            Lang::Or(_, _, h) => h,
            Lang::Union(_, _, h) => h,
            Lang::In(_, _, h) => h,
            Lang::Add(_, _, h) => h,
            Lang::Eq(_, _, h) => h,
            Lang::Eq2(_, _, h) => h,
            Lang::NotEq(_, _, h) => h,
            Lang::Modu(_, _, h) => h,
            Lang::Modu2(_, _, h) => h,
            Lang::LesserThan(_, _, h) => h,
            Lang::GreaterThan(_, _, h) => h,
            Lang::LesserOrEqual(_, _, h) => h,
            Lang::GreaterOrEqual(_, _, h) => h,
            Lang::Chain(_, _, h) => h,
            Lang::Scope(_, h) => h,
            Lang::Function(_, _, _, _, h) => h,
            Lang::Module(_, _, h) => h,
            Lang::ModuleDecl(_, h) => h,
            Lang::Variable(_, _, _, _, _, h) => h,
            Lang::FunctionApp(_, _, h) => h,
            Lang::ArrayIndexing(_, _, h) => h,
            Lang::Let(_, _, _, h) => h,
            Lang::Array(_, h) => h,
            Lang::Record(_, h) => h,
            Lang::Alias(_, _, _, h) => h,
            Lang::Tag(_, _, h) => h,
            Lang::If(_, _, _, h) => h,
            Lang::Match(_, _, h) => h,
            Lang::Tuple(_, h) => h,
            Lang::Sequence(_, h) => h,
            Lang::Assign(_, _, h) => h,
            Lang::Comment(_, h) => h,
            Lang::Range(_, _, _, h) => h,
            Lang::ModImp(_, h) => h,
            Lang::Import(_, h) => h,
            Lang::Header(_, h) => h,
            Lang::GenFunc(_, _, h) => h,
            Lang::Test(_, h) => h,
            Lang::Return(_, h) => h,
            Lang::VecBloc(_, h) => h,
            Lang::Lambda(_, h) => h,
            Lang::Library(_, h) => h,
            Lang::Exp(_, h) => h,
            Lang::Any(h) => h,
            Lang::Empty(h) => h,
            Lang::Signature(_, _, h) => h,
            Lang::ForLoop(_, _, _, h) => h,
            Lang::RFunction(_, _, h) => h,
        }.clone()
    }

    pub fn to_dependent_type(&self) -> Option<Type> {
        match self {
            //todo: if the user give a negative index as dependent value type
            Lang::Integer(i, h) 
                => Some(Type::Integer((*i).into(), h.clone())),
            Lang::Char(c, h) 
                => Some(Type::Char(c.to_owned().into(), h.clone())),
            _ => None
        }
    }

    pub fn linearize_array(&self) -> Vec<Lang> {
        match self {
            Lang::Array(v, _) 
                => v.iter()
                .fold(vec![], |acc, x| 
                      acc.iter()
                      .chain(x.linearize_array().iter())
                      .cloned().collect()),
            _ => vec![self.to_owned()]
        }
    }

    pub fn is_empty_scope(&self) -> bool {
        if let Lang::Scope(body, _) = self {
            if body.len() == 1 {
                if let Lang::Empty(_) = body[0] {
                    true
                } else { false }
            } else { false }
        } else { false }
    } 

    pub fn is_r_function(&self) -> bool {
        match self {
            Lang::RFunction(_, _, _) => true,
            _ => false
        }
    }

    pub fn nb_params(&self) -> usize {
        match self {
            Lang::Function(_, params, _, _, _) => params.len(),
            _ => 0 as usize
        }
    }

    pub fn simple_print(&self) -> String {
        match self {
            Lang::Number(_, _) => "Number".to_string(),
            Lang::Integer(_, _) => "Integer".to_string(),
            Lang::Char(_, _) => "Char".to_string(),
            Lang::Bool(_, _) => "Bool".to_string(),
            Lang::And(_, _, _) => "And".to_string(),
            Lang::Or(_, _, _) => "Or".to_string(),
            Lang::Union(_, _, _) => "Union".to_string(),
            Lang::In(_, _, _) => "In".to_string(),
            Lang::Add(_, _, _) => "Add".to_string(),
            Lang::Eq(_, _, _) => "Eq".to_string(),
            Lang::Eq2(_, _, _) => "Eq2".to_string(),
            Lang::NotEq(_, _, _) => "NotEq".to_string(),
            Lang::Modu(_, _, _) => "Modu".to_string(),
            Lang::Modu2(_, _, _) => "Modu2".to_string(),
            Lang::LesserThan(_, _, _) => "LesserThan".to_string(),
            Lang::GreaterThan(_, _, _) => "GreaterThan".to_string(),
            Lang::LesserOrEqual(_, _, _) => "LesserOrEqual".to_string(),
            Lang::GreaterOrEqual(_, _, _) => "GreatOrEqual".to_string(),
            Lang::Chain(_, _, _) => "Chain".to_string(),
            Lang::Scope(_, _) => "Scope".to_string(),
            Lang::Function(_, _, _, _, _) => "Function".to_string(),
            Lang::Module(_, _, _) => "Module".to_string(),
            Lang::ModuleDecl(_, _) => "ModuleDecl".to_string(),
            Lang::Variable(name, _, _, _, _, _) => format!("Variable({})", name),
            Lang::FunctionApp(var, _, _) => 
                format!("FunctionApp({})", Var::from_language(*(var.clone())).unwrap().get_name()),
            Lang::ArrayIndexing(_, _, _) => "ArrayIndexing".to_string(),
            Lang::Let(_, _, _, _) => "Let".to_string(),
            Lang::Array(_, _) => "Array".to_string(),
            Lang::Record(_, _) => "Record".to_string(),
            Lang::Alias(_, _, _, _) => "Alias".to_string(),
            Lang::Tag(_, _, _) => "Tag".to_string(),
            Lang::If(_, _, _, _) => "If".to_string(),
            Lang::Match(_, _, _) => "Match".to_string(),
            Lang::Tuple(_, _) => "Tuple".to_string(),
            Lang::Sequence(_, _) => "Sequence".to_string(),
            Lang::Assign(_, _, _) => "Addign".to_string(),
            Lang::Comment(_, _) => "Comment".to_string(),
            Lang::Range(_, _, _, _) => "Range".to_string(),
            Lang::ModImp(_, _) => "ModImp".to_string(),
            Lang::Import(_, _) => "Import".to_string(),
            Lang::Header(_, _) => "Header".to_string(),
            Lang::GenFunc(_, _, _) => "GenFunc".to_string(),
            Lang::Test(_, _) => "Test".to_string(),
            Lang::Return(_, _) => "Return".to_string(),
            Lang::VecBloc(_, _) => "VecBloc".to_string(),
            Lang::Lambda(_, _) => "Lambda".to_string(),
            Lang::Library(_, _) => "Library".to_string(),
            Lang::Exp(_, _) => "Exp".to_string(),
            Lang::Any(_) => "Any".to_string(),
            Lang::Empty(_) => "Empty".to_string(),
            Lang::Signature(_, _, _) => "Signature".to_string(),
            Lang::ForLoop(_, _, _, _) => "ForLoop".to_string(),
            Lang::RFunction(_, _, _) => "RFunction".to_string(),
        }
    }
    
}

fn typescript_type(s: &str, cont: &Context) -> String {
    match s {
        "integer" => "number".to_string(),
        "number" => "number".to_string(),
        "bool" => "boolean".to_string(),
        x => {
            let typ = cont.get_type_from_class(x);
            match typ {
                Type::Record(body, _) => {
                    let res = body.iter()
                        .map(|at| at.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{{ {} }}", res)
                },
                _ => format!("check typescript_type for: {}", typ)
            }
        }
    }
}

fn wasm_type(s: &str) -> String {
    match s {
        "integer" => "i32".to_string(),
        x => format!("Check `wasm_type` function for: {}", x)
    }
}

impl From<Lang> for HelpData {
   fn from(val: Lang) -> Self {
       match val {
           Lang::Number(_, h) => h,
           Lang::Integer(_, h) => h,
           Lang::Bool(_, h) => h,
           Lang::Char(_, h) => h,
           Lang::Variable(_, _, _, _, _, h) => h,
           Lang::Match(_, _, h) => h,
           Lang::FunctionApp(_, _, h) => h,
           Lang::Empty(h) => h,
           Lang::Array(_, h) => h,
           Lang::Eq(_, _, h) => h,
           Lang::Eq2(_, _, h) => h,
           Lang::NotEq(_, _, h) => h,
           Lang::Chain(_, _, h) => h,
           Lang::Record(_, h) => h,
           Lang::Scope(_, h) => h,
           Lang::Let(_, _, _, h) => h,
           Lang::Alias(_, _, _, h) => h,
           Lang::Lambda(_, h) => h,
           Lang::Function(_, _, _, _, h) => h,
           Lang::VecBloc(_, h) => h,
           Lang::If(_, _, _, h) => h,
           Lang::Assign(_, _, h) => h,
           Lang::And(_, _, h) => h,
           Lang::Or(_, _, h) => h,
           Lang::Union(_, _, h) => h,
           Lang::In(_, _, h) => h,
           Lang::Add(_, _, h) => h,
           Lang::Modu(_, _, h) => h,
           Lang::Modu2(_, _, h) => h,
           Lang::Module(_, _, h) => h,
           Lang::ModuleDecl(_, h) => h,
           Lang::ModImp(_, h) => h,
           Lang::Import(_, h) => h,
           Lang::Header(_, h) => h,
           Lang::GreaterThan(_, _, h) => h,
           Lang::GreaterOrEqual(_, _, h) => h,
           Lang::LesserThan(_, _, h) => h,
           Lang::LesserOrEqual(_, _, h) => h,
           Lang::ArrayIndexing(_, _, h) => h,
           Lang::Tag(_, _, h) => h,
           Lang::Tuple(_, h) => h,
           Lang::Sequence(_, h) => h,
           Lang::Comment(_, h) => h,
           Lang::Range(_, _, _, h) => h,
           Lang::GenFunc(_, _, h) => h,
           Lang::Test(_, h) => h,
           Lang::Return(_, h) => h,
           Lang::Library(_, h) => h,
           Lang::Exp(_, h) => h,
           Lang::Any(h) => h,
           Lang::Signature(_, _, h) => h,
           Lang::ForLoop(_, _, _, h) => h,
           Lang::RFunction(_, _, h) => h,
       }.clone()
   } 
}

impl From<Vec<Lang>> for HelpData {
   fn from(val: Vec<Lang>) -> Self {
       if val.len() > 0 {
            val[0].clone().into()
       } else { HelpData::default() }
   } 
}

use std::fmt;
impl fmt::Display for Lang {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            Lang::Variable(name, path, _permision, _bo, typ, _h) 
                => format!("{}{} -> {}", path, name, typ),
            _ => format!("{:?}", self)
        };
        write!(f, "{}", res)       
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::single_element;

    #[test]
    fn test_empty_scope(){
        let res = single_element("fn(): bool { ... }".into()).unwrap().1;
        if let Lang::Function(_, _, _, body, _) = res {
            assert!(body.is_empty_scope());
        } else { assert!(false, "This expression is not a function"); }
    }
}
