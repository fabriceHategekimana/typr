#![allow(dead_code)]
use serde::Serialize;
use crate::argument_type::ArgumentType;
use crate::argument_kind::ArgumentKind;
use crate::tag::Tag;
use crate::context::generate_arg;
use crate::kind::Kind;
use std::collections::HashSet;
use crate::help_data::HelpData;
use crate::type_printer::format;
use std::fmt;
use crate::path::Path;
use crate::tint::Tint;
use crate::tchar::Tchar;
use crate::function_type::FunctionType;
use crate::type_comparison::has_generic_label;
use crate::type_comparison::all_subtype2;
use crate::type_comparison::contains_all2;
use std::cmp::Ordering;
use crate::nominal_context::TypeCategory;
use crate::Context;
use crate::type_comparison::reduce_type;
use std::hash::Hash;
use std::hash::Hasher;
use crate::builder;

fn to_string<T: ToString>(v: &[T]) -> String {
    let res = v.iter()
        .map(|x| x.to_string())
        .reduce(|acc, x| format!("{}, {}", acc, x))
        .unwrap_or("".to_string());
    format!("[{}]", res)
}



#[derive(Debug, Clone, Serialize, Eq)]
pub enum Type {
    Number(HelpData),
    Integer(Tint, HelpData),
    Boolean(HelpData),
    Char(Tchar, HelpData),
    Embedded(Box<Type>, HelpData),
    Function(Vec<ArgumentKind>, Vec<Type>, Box<Type>, HelpData),
    Generic(String, HelpData),
    IndexGen(String, HelpData),
    LabelGen(String, HelpData),
    Array(Box<Type>, Box<Type>, HelpData),
    Record(Vec<ArgumentType>, HelpData),
    Alias(String, Vec<Type>, Path, bool, HelpData), //for opacity
    Tag(String, Box<Type>, HelpData),
    Union(Vec<Tag>, HelpData),
    Interface(Vec<ArgumentType>, HelpData),
    Params(Vec<Type>, HelpData),
    Add(Box<Type>, Box<Type>, HelpData),
    Mul(Box<Type>, Box<Type>, HelpData),
    Minus(Box<Type>, Box<Type>, HelpData),
    Div(Box<Type>, Box<Type>, HelpData),
    Failed(String, HelpData),
    Opaque(String, HelpData),
    Multi(Box<Type>, HelpData),
    Tuple(Vec<Type>, HelpData),
    If(Box<Type>, Vec<Type>, HelpData),
    Condition(Box<Type>, Box<Type>, Box<Type>, HelpData),
    In(HelpData),
    DataFrame(HelpData),
    RFunction(HelpData),
    Empty(HelpData),
    Any(HelpData)
}

//main
impl Type {
    pub fn type_extraction(&self) -> Vec<Type> {
        match self {
            Type::Function(_, args, ret, _)
                => {
                    let mut sol = args.clone();
                    sol.push((**ret).clone());
                    sol.push(self.clone()); sol
                }
            Type::Union(tags, _) => {
               let mut sol = tags.iter().map(|tag| tag.to_type()).collect::<Vec<_>>();
               sol.push(self.clone()); sol
            },
            typ => vec![typ.clone()]
        }.iter()
        .map(|typ| typ.add_kinds_in_functions_if_not())
        .collect()
    }

    pub fn get_label(&self) -> String {
        match self {
            Type::Char(l, _) => l.to_string(),
            Type::LabelGen(l, _) => l.to_string(),
            _ => panic!("The type {} wasn't a label", self)
        }
    }

    fn add_kinds_in_functions_if_not(&self) -> Type {
        match self {
            Type::Function(_kinds, args, ret, h) => {
                let new_kinds = args.iter()
                    .chain([(**ret).clone()].iter())
                    .flat_map(|typ| typ.extract_generics())
                    .collect::<HashSet<_>>()
                    .iter()
                    .map(|typ| ArgumentKind::from((typ.clone(), typ.get_kind())))
                    .collect::<Vec<_>>();
                Type::Function(new_kinds, args.clone(), ret.clone(), h.clone())
            },
            typ => typ.clone()
        }
    }

    pub fn replace_function_types(self: Type, t1: Type, t2: Type) -> Type {
        match self {
            Type::Function(kinds, args, ret, h) => {
                let new_args = args.iter()
                    .map(|typ| if *typ == t1 { t2.clone() } else {t1.clone()})
                    .collect::<Vec<_>>();
                let new_ret = if *ret == t1 { t2 } else { *ret };
                Type::Function(kinds.clone(), new_args, Box::new(new_ret), h)
            },
            _ => self
        }
    }

    pub fn without_embeddings(self) -> Type {
        match self {
            Type::Record(args, h) => {
                let new_args = args.iter()
                    .map(|arg| arg.remove_embeddings())
                    .collect();
                Type::Record(new_args, h.clone())
            },
            typ => typ
        }
    }

    pub fn to_typescript(&self) -> String {
       match self {
           Type::Boolean(_) => "boolean".to_string(),
           Type::Integer(_, _) => "number".to_string(),
           Type::Number(_) => "number".to_string(),
           Type::Char(_, _) => "string".to_string(),
           Type::Record(body, _) => {
                let res = body.iter()
                    .map(|at| format!("{}: {}", at.get_argument(), at.get_type().to_typescript()))
                    .collect::<Vec<_>>().join(", ");
                format!("{{ {} }}", res)
           },
           Type::Array(_size, body, _) => format!("{}[]", body.to_typescript()),
           Type::IndexGen(id, _) => id.to_uppercase(),
           Type::Generic(val, _) => val.to_uppercase(), 
           Type::Function(kinds, args, ret, _) => {
               let res = args.iter()
                    .enumerate()
                    .map(|(i, typ)| format!("{}: {}", generate_arg(i), typ.to_typescript()))
                    .collect::<Vec<_>>().join(", ");
               if kinds.len() == 0 {
                   format!("({}) => {}", res, ret.to_typescript())
               } else {
                   let res2 = kinds.iter()
                       .map(|ak| ak.get_argument().to_typescript())
                       .collect::<Vec<_>>()
                       .join(", ");
                   format!("<{}>({}) => {}", res2, res, ret.to_typescript())
               }
           },
           Type::Tag(name, typ, _) 
               => format!("{{ _type: '{}',  _body: {} }}", name, typ.to_typescript()),
           Type::Any(_) => "null".to_string(),
           Type::Empty(_) => "null".to_string(),
           Type::Add(_,_, _) => "T".to_string(),
           Type::Minus(_,_, _) => "T".to_string(),
           Type::Div(_,_, _) => "T".to_string(),
           Type::Mul(_,_, _) => "T".to_string(),
           _ => format!("the type: {} is not yet in to_typescript()", self)
       } 
    }

    pub fn to_assemblyscript(&self) -> String {
       match self {
           Type::Boolean(_) => "bool".to_string(),
           Type::Integer(_, _) => "i32".to_string(),
           Type::Number(_) => "f64".to_string(),
           Type::Char(_, _) => "string".to_string(),
           Type::Record(body, _) => {
                let res = body.iter()
                    .map(|at| format!("{}: {}", at.get_argument(), at.get_type().to_typescript()))
                    .collect::<Vec<_>>().join(", ");
                format!("{{ {} }}", res)
           },
           Type::Array(_size, body, _) => format!("{}[]", body.to_typescript()),
           Type::IndexGen(id, _) => id.to_uppercase(),
           Type::Generic(val, _) => val.to_uppercase(), 
           Type::Function(kinds, args, ret, _) => {
               let res = args.iter()
                    .enumerate()
                    .map(|(i, typ)| format!("{}: {}", generate_arg(i), typ.to_typescript()))
                    .collect::<Vec<_>>().join(", ");
               if kinds.len() == 0 {
                   format!("({}) => {}", res, ret.to_typescript())
               } else {
                   let res2 = kinds.iter()
                       .map(|ak| ak.get_argument().to_typescript())
                       .collect::<Vec<_>>()
                       .join(", ");
                   format!("<{}>({}) => {}", res2, res, ret.to_typescript())
               }
           },
           Type::Tag(name, typ, _) => format!("{{ _type: '{}',  _body: {} }}", name, typ.to_typescript()),
           Type::Any(_) => "null".to_string(),
           Type::Empty(_) => "null".to_string(),
           Type::Add(_,_, _) => "T".to_string(),
           Type::Minus(_,_, _) => "T".to_string(),
           Type::Div(_,_, _) => "T".to_string(),
           Type::Mul(_,_, _) => "T".to_string(),
           _ => format!("the type: {} is not yet in to_typescript()", self)
       } 
    }

    pub fn extract_generics(&self) -> Vec<Type> {
        match self {
            Type::Generic(_, _) | Type::IndexGen(_, _) | Type::LabelGen(_, _)
                => vec![self.clone()],
            Type::Function(kinds, args, ret_typ, _) => {
                kinds.iter()
                    .map(|ak| ak.get_argument())
                    .chain(args.iter().cloned())
                    .chain([(**ret_typ).clone()].iter().cloned())
                    .collect::<HashSet<_>>()
                    .iter().flat_map(|typ| typ.extract_generics())
                    .collect::<Vec<_>>()
            }
            Type::Array(ind, typ, _) => {
               typ.extract_generics() 
                   .iter()
                   .chain(ind.extract_generics().iter())
                   .collect::<HashSet<_>>()
                   .into_iter().cloned().collect::<Vec<_>>()
            },
                Type::Record(v, _) => {
                   v.iter() 
                       .flat_map(|argt| 
                                 [argt.get_argument(), argt.get_type()])
                       .flat_map(|typ| typ.extract_generics())
                       .collect::<HashSet<_>>()
                       .into_iter().collect::<Vec<_>>()
                }
            _ => vec![]
        }
    }

    pub fn get_kind(&self) -> Kind {
        match self {
            Type::IndexGen(_, _) | Type::Integer(_, _) => Kind::Dim,
            _ => Kind::Type
        }
    }

    pub fn index_calculation(&self) -> Type {
        match self {
            Type::Add(a, b, _) 
                => a.index_calculation().sum_index(&b.index_calculation()),
            Type::Minus(a, b, _) 
                => a.index_calculation().minus_index(&b.index_calculation()),
            Type::Mul(a, b, _) 
                => a.index_calculation().mul_index(&b.index_calculation()),
            Type::Div(a, b, _) 
                => a.index_calculation().div_index(&b.index_calculation()),
            Type::Array(ind, typ, h) =>
                Type::Array(
                    Box::new(ind.index_calculation()), 
                    Box::new(typ.index_calculation()),
                    h.clone()),
                    Type::Function(_kinds, args, ret_typ, h) => {
                        let new_args = args.iter()
                            .map(|typ| typ.index_calculation())
                            .collect::<Vec<_>>();
                        let new_kinds = args.iter()
                            .flat_map(|typ| typ.extract_generics())
                            .collect::<HashSet<_>>().iter()
                            .map(|typ| ArgumentKind::from((typ.clone(), typ.get_kind())))
                            .collect::<Vec<_>>();
                        Type::Function(
                            new_kinds,
                            new_args,
                            Box::new(ret_typ.index_calculation()), h.clone())
                    }
            _ => self.clone()
        }
    }

    fn sum_index(&self, i: &Type) -> Type {
        match (self, i) {
            (Type::Integer(a, h), Type::Integer(b, _)) => Type::Integer(*a+*b, h.clone()),
            (Type::Record(a, h), Type::Record(b, _)) 
                => Type::Record(
                    a.iter()
                    .chain(b.iter())
                    .cloned()
                    .collect::<Vec<_>>(), h.clone()),
            _ => panic!("Type {} and {} can't be added", self, i)
        }
    }

    fn minus_index(&self, i: &Type) -> Type {
        match (self, i) {
            (Type::Integer(a, h), Type::Integer(b, _)) => Type::Integer(*a-*b, h.clone()),
            _ => panic!("Type {} and {} can't be added", self, i)
        }
    }

    fn mul_index(&self, i: &Type) -> Type {
        match (self, i) {
            (Type::Integer(a, h), Type::Integer(b, _)) => Type::Integer(*a*(*b), h.clone()),
            _ => panic!("Type {} and {} can't be added", self, i)
        }
    }

    fn div_index(&self, i: &Type) -> Type {
        match (self, i) {
            (Type::Integer(a, h), Type::Integer(b, _)) => Type::Integer(*a/(*b), h.clone()),
            _ => panic!("Type {} and {} can't be added", self, i)
        }
    }

    pub fn get_shape(&self) -> Option<String> {
        if let Type::Array(i, t, _) = self {
            match (*i.clone(), t.get_shape()) {
                (Type::IndexGen(_, _), _) => Some("dim(===)".to_string()),
                (Type::Integer(j, _), Some(rest)) => Some(format!("{}, {}", j, rest)),
                (Type::Integer(j, _), None) => Some(format!("{}", j)),
                _ => None
            }
        } else { None }
    }

    pub fn to_function_type(&self) -> Option<FunctionType> {
        match self {
            Type::Function(kinds, args, ret_ty, h) 
                => Some(FunctionType(kinds.clone(), args.clone(), (**ret_ty).clone(), h.clone())),
            Type::RFunction(h) 
                => Some(FunctionType(vec![], vec![], builder::empty_type(), h.clone())),
            _ => None
        }
    }

    pub fn get_type_pattern(&self) -> Option<ArgumentType> {
        if let Type::Record(fields, _) = self {
            (fields.len() == 1)
                .then(|| fields[0].clone())
        } else {None}
    }

    pub fn is_boolean(&self) -> bool {
        if let Type::Boolean(_) = self {
            true
        } else { false }
    }

    pub fn dependent_type(&self, dep_typ: &Type) -> bool {
        match (dep_typ, self) {
            (Type::Integer(_, _), Type::IndexGen(_, _)) => true,
            (Type::Char(_, _), Type::LabelGen(_, _)) => true,
            _ => false
        }
    }

    pub fn pretty(&self) -> String {
        format(self)
    }

    pub fn is_tag_or_union(&self) -> bool {
        match self {
            Type::Tag(_, _, _) => true,
            Type::Union(_, _) => true,
            _ => false
        }
    }

    pub fn is_generic(&self) -> bool {
        match self {
            Type::Generic(_, _) => true,
            Type::IndexGen(_, _) => true,
            Type::LabelGen(_, _) => true,
            _ => false
        }
    }

    pub fn add_path(self, path: Path) -> Type {
        match self.clone() {
            Type::Alias(name, params, path2, opacity, h) 
                => {
                    Type::Alias(name, params, path2.add_path(path), opacity, h)
                },
            _ => self.to_owned()
        }
    }

    pub fn exact_match(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Integer(a, _), Type::Integer(b, _)) => a == b,
            (Type::Char(a, _), Type::Char(b, _)) => a == b,
            _ => self == other
        }
    }

    pub fn for_var(self) -> Type {
        match self.to_owned() {
            Type::Function(_k, p, _r, _h) => {
                if p.len() > 0 {
                   p[0].to_owned()
                } else {
                    self
                }
            },
            t => t
        }
    }

    pub fn is_subtype(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Empty(_), _) => true,
            (typ1, typ2) if typ1 == typ2 => true,
            // Array subtyping
            (_, Type::Any(_)) => true,
            (Type::Array(n1, t1, _), Type::Array(n2, t2, _)) => {
                n1.is_subtype(n2) && t1.is_subtype(t2)
            },
            (Type::Function(_, args1, ret_typ1, _), Type::Function(_, args2, ret_typ2, _)) => {
                args1.iter().chain([&(**ret_typ1)])
                    .zip(args2.iter().chain([&(**ret_typ2)]))
                    .all(|(typ1, typ2)| typ1.is_subtype(typ2))
            }
            // Interface subtyping
            (_type1, Type::Interface(_args, _)) => {
                todo!();
            }

            // Record subtyping
            (Type::Record(r1, _), Type::Record(r2, _)) => {
                if has_generic_label(r2) && (r1.len() == r2.len()) {
                    all_subtype2(r1, r2)
                } else if let Some(_arg_typ) = other.get_type_pattern() {
                    true
                } else {
                    contains_all2(r1, r2)
                }
            },

            (Type::Union(types1, _), Type::Union(_types2, _)) => {
                types1.iter().all(|t1| t1.to_type().is_subtype(other))
            },

            // Union subtyping
            (Type::Tag(_name, _body, _h), Type::Union(types, _)) => {
                types.iter().any(|t| self.is_subtype(&t.to_type()))
            },
            (Type::Tag(name1, body1, _h1), Type::Tag(name2, body2, _h2)) => {
                (name1 == name2) && body1.is_subtype(body2)
            },

            // Generic subtyping
            (_, Type::Generic(_, _)) => true,
            (Type::Integer(_, _), Type::IndexGen(_, _)) => true,
            (Type::Char(_, _), Type::LabelGen(_, _)) => true,
            (Type::IndexGen(_, _), Type::IndexGen(_, _)) => true,

            // Params subtyping
            (Type::Params(p1, _), Type::Params(p2, _)) => {
                p1.len() == p2.len() && 
                p1.iter().zip(p2.iter()).all(|(t1, t2)| t1.is_subtype(t2))
            }

            _ => false
        }
    }
    
    pub fn to_category(&self) -> TypeCategory {
        match self {
            Type::Array(_, _, _) => TypeCategory::Array,
            Type::Function(_, _, _, _) => TypeCategory::Function,
            Type::Record(_, _) => TypeCategory::Record,
            Type::Tag(_, _, _) => TypeCategory::Tag,
            Type::Union(_, _) => TypeCategory::Union,
            Type::Interface(_, _) => TypeCategory::Interface,
            Type::Boolean(_) => TypeCategory::Boolean,
            Type::Number(_) => TypeCategory::Number,
            Type::Char(_, _) => TypeCategory::Char,
            Type::Generic(_, _) => TypeCategory::Generic,
            Type::IndexGen(_, _) => TypeCategory::Generic,
            Type::LabelGen(_, _) => TypeCategory::Generic,
            _ => TypeCategory::Rest
        }
    }

    // for removing litteral data for Integer and Char
    pub fn generalize(self) -> Type {
        match self {
            Type::Integer(Tint::Val(_), h) 
                => Type::Integer(Tint::Unknown, h),
            Type::Char(Tchar::Val(_), h) 
                => Type::Char(Tchar::Unknown, h),
            t => t
        }
    }

    pub fn reduce(&self, context: &Context) -> Type {
        reduce_type(context, self)
    }

    fn get_type_shape(&self) -> usize {
        match self {
            Type::Function(_, v, _, _) => v.len(),
            Type::Tuple(v, _) => v.len(),
            _ => 0 as usize
        }
    }

    pub fn same_shape(&self, other: &Type) -> bool {
        self.get_type_shape() == other.get_type_shape()
    }

    pub fn get_help_data(&self) -> HelpData {
        match self {
            Type::Integer(_, h) => h.clone(),
            Type::Number(h) => h.clone(),
            Type::Char(_, h) => h.clone(),
            Type::Boolean(h) => h.clone(),
            Type::Embedded(_, h) => h.clone(),
            Type::Function(_, _, _, h) => h.clone(),
            Type::Generic(_, h) => h.clone(),
            Type::IndexGen(_, h) => h.clone(),
            Type::LabelGen(_, h) => h.clone(),
            Type::Array(_, _, h) => h.clone(),
            Type::Record(_, h) => h.clone(),
            Type::Alias(_, _, _, _, h) => h.clone(),
            Type::Tag(_, _, h) => h.clone(),
            Type::Union(_, h) => h.clone(),
            Type::Interface(_, h) => h.clone(),
            Type::Params(_, h) => h.clone(),
            Type::Add(_, _, h) => h.clone(),
            Type::Mul(_, _, h) => h.clone(),
            Type::Minus(_, _, h) => h.clone(),
            Type::Div(_, _, h) => h.clone(),
            Type::Failed(_, h) => h.clone(),
            Type::Opaque(_, h) => h.clone(),
            Type::Multi(_, h) => h.clone(),
            Type::Tuple(_, h) => h.clone(),
            Type::If(_, _, h) => h.clone(),
            Type::Condition(_, _, _, h) => h.clone(),
            Type::In(h) => h.clone(),
            Type::DataFrame(h) => h.clone(),
            Type::RFunction(h) => h.clone(),
            Type::Empty(h) => h.clone(),
            Type::Any(h) => h.clone(),
        }
    }

    pub fn set_help_data(self, h2: HelpData) -> Type {
        match self {
            Type::Integer(i, _) => Type::Integer(i, h2),
            Type::Number(_) => Type::Number(h2),
            Type::Char(a, _) => Type::Char(a, h2),
            Type::Boolean(_) => Type::Boolean(h2),
            Type::Embedded(a, _) => Type::Embedded(a, h2),
            Type::Function(a1, a2, a3, _) => Type::Function(a1, a2, a3, h2),
            Type::Generic(a, _) => Type::Generic(a, h2),
            Type::IndexGen(a, _) => Type::IndexGen(a, h2),
            Type::LabelGen(a, _) => Type::LabelGen(a, h2),
            Type::Array(a1, a2, _) => Type::Array(a1, a2, h2),
            Type::Record(a, _) => Type::Record(a, h2),
            Type::Alias(a1, a2, a3, a4, _) => Type::Alias(a1, a2, a3, a4, h2),
            Type::Tag(a1, a2, _) => Type::Tag(a1, a2, h2),
            Type::Union(a, _) => Type::Union(a, h2),
            Type::Interface(a, _) => Type::Interface(a, h2),
            Type::Params(a, _) => Type::Params(a, h2),
            Type::Add(a1, a2, _) => Type::Add(a1, a2, h2),
            Type::Mul(a1, a2, _) => Type::Mul(a1, a2, h2),
            Type::Minus(a1, a2, _) => Type::Minus(a1, a2, h2),
            Type::Div(a1, a2, _) => Type::Div(a1, a2, h2),
            Type::Failed(a, _) => Type::Failed(a, h2),
            Type::Opaque(a, _) => Type::Opaque(a, h2),
            Type::Multi(a, _) => Type::Multi(a, h2),
            Type::Tuple(a, _) => Type::Tuple(a, h2),
            Type::If(a1, a2, _) => Type::If(a1, a2, h2),
            Type::Condition(a1, a2, a3, _) => Type::Condition(a1, a2, a3, h2),
            Type::In(_) => Type::In(h2),
            Type::DataFrame(_) => Type::DataFrame(h2),
            Type::RFunction(_) => Type::RFunction(h2),
            Type::Empty(_) => Type::Empty(h2),
            Type::Any(_) => Type::Any(h2) 
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Type::Empty(_) => true,
            _ => false
        }
    }

    pub fn to_array(&self) -> Option<Array> {
        match self {
            Type::Array(t1, t2, h) 
                => Some(Array{index: (**t1).clone(), base_type: (**t2).clone(), help_data: h.clone()}),
            _ => None
        }
    }

}

pub struct Array {
    pub index: Type,
    pub base_type: Type,
    pub help_data: HelpData
}


impl From<Vec<Type>> for HelpData {
   fn from(val: Vec<Type>) -> Self {
        if val.len() > 0 {
            val[0].clone().into()
        } else { HelpData::default() }
   } 
}

impl From<Type> for HelpData {
   fn from(val: Type) -> Self {
       match val {
           Type::Empty(h) => h,
           Type::Generic(_, h) => h,
           Type::IndexGen(_, h) => h,
           Type::Minus(_, _, h) => h,
           Type::Add(_, _, h) => h,
           Type::Mul(_, _, h) => h,
           Type::Div(_, _, h) => h,
           Type::Tag(_, _, h) => h,
           Type::Function(_, _, _, h) => h,
           Type::Char(_, h) => h,
           Type::Integer(_, h) => h,
           Type::Record(_, h) => h,
           Type::Boolean(h) => h,
           Type::Array(_, _, h) => h,
           Type::Number(h) => h,
           e => panic!("The type element {:?} is not yet implemented", e)
       }.clone()
   } 
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Number(_), Type::Number(_)) => true,
            (Type::Integer(_, _), Type::Integer(_, _)) => true,
            (Type::Boolean(_), Type::Boolean(_)) => true,
            (Type::Char(_, _), Type::Char(_, _)) => true, 
            (Type::Embedded(e1, _), Type::Embedded(e2, _)) => e1 == e2,
            (Type::Function(a1, b1, c1, _), Type::Function(a2, b2, c2, _)) 
                => a1 == a2 && b1 == b2 && c1 == c2,
            (Type::Generic(e1, _), Type::Generic(e2, _)) => e1 == e2,
            (Type::IndexGen(e1, _), Type::IndexGen(e2, _)) => e1 == e2,
            (Type::LabelGen(e1, _), Type::LabelGen(e2, _)) => e1 == e2,
            (Type::Array(a1, b1, _), Type::Array(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Record(e1, _), Type::Record(e2, _)) => e1 == e2,
            (Type::Alias(a1, b1, c1, d1, _), Type::Alias(a2, b2, c2, d2, _)) 
                => a1 == a2 && b1 == b2 && c1 == c2 && d1 == d2,
            (Type::Tag(a1, b1, _), Type::Tag(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Union(e1, _), Type::Union(e2, _)) => e1 == e2,
            (Type::Interface(e1, _), Type::Interface(e2, _)) => e1 == e2,
            (Type::Params(e1, _), Type::Params(e2, _)) => e1 == e2,
            (Type::Add(a1, b1, _), Type::Add(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Minus(a1, b1, _), Type::Minus(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Mul(a1, b1, _), Type::Mul(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Div(a1, b1, _), Type::Div(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Failed(e1, _), Type::Failed(e2, _)) => e1 == e2,
            (Type::Opaque(e1, _), Type::Opaque(e2, _)) => e1 == e2,
            (Type::Multi(e1, _), Type::Multi(e2, _)) => e1 == e2,
            (Type::Tuple(e1, _), Type::Tuple(e2, _)) => e1 == e2,
            (Type::If(a1, b1, _), Type::If(a2, b2, _)) 
                => a1 == a2 && b1 == b2,
            (Type::Condition(a1, b1, c1, _), Type::Condition(a2, b2, c2, _)) 
                => a1 == a2 && b1 == b2 && c1 == c2,
            (Type::In(_), Type::In(_)) => true,
            (Type::Empty(_), Type::Empty(_)) => true,
            (Type::Any(_), Type::Any(_)) => true,
            (Type::DataFrame(_), Type::DataFrame(_)) => true,
            _ => false
        }
    }
}

impl fmt::Display for Type {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Function(_k, p, r, h) 
                => write!(f, "({}) -> {}", Type::Params(p.clone(), h.clone()), r),
            Type::Params(v, _) => {
                let res = v.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ");
                write!(f, "{}", res)
            }
            _ => write!(f, "{}", format(self))
        }
    }
}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_subtype(other) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.is_subtype(other) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Utiliser un discriminant pour différencier les variantes
        match self {
            Type::Number(_) => 0.hash(state),
            Type::Integer(_, _) => 1.hash(state),
            Type::Boolean(_) => 2.hash(state),
            Type::Char(_, _) => 3.hash(state),
            Type::Embedded(_, _) => 4.hash(state),
            Type::Function(_, _, _, _) => 5.hash(state),
            Type::Generic(_, _) => 6.hash(state),
            Type::IndexGen(_, _) => 7.hash(state),
            Type::LabelGen(_, _) => 8.hash(state),
            Type::Array(_, _, _) => 9.hash(state),
            Type::Record(_, _) => 10.hash(state),
            Type::Alias(_, _, _, _, _) => 11.hash(state),
            Type::Tag(_, _, _) => 12.hash(state),
            Type::Union(_, _) => 13.hash(state),
            Type::Interface(_, _) => 14.hash(state),
            Type::Params(_, _) => 15.hash(state),
            Type::Add(_, _, _) => 16.hash(state),
            Type::Minus(_, _, _) => 17.hash(state),
            Type::Mul(_, _, _) => 18.hash(state),
            Type::Div(_, _, _) => 19.hash(state),
            Type::Failed(_, _) => 20.hash(state),
            Type::Opaque(_, _) => 21.hash(state),
            Type::Multi(_, _) => 22.hash(state),
            Type::Tuple(_, _) => 23.hash(state),
            Type::If(_, _, _) => 24.hash(state),
            Type::Condition(_, _, _, _) => 25.hash(state),
            Type::In(_) => 26.hash(state),
            Type::Empty(_) => 27.hash(state),
            Type::Any(_) => 28.hash(state),
            Type::DataFrame(_) => 29.hash(state),
            Type::RFunction(_) => 29.hash(state),
        }
    }
}
