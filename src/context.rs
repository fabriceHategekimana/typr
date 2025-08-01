#![allow(dead_code)]
use crate::r#type::Type;
use crate::language::Lang;
use crate::var::Var;
use crate::kind::Kind;
use crate::nominal_context::TypeNominal;
use crate::argument_type::ArgumentType;
use crate::vartype::VarType;
use crate::type_comparison;
use crate::Environment;
use crate::type_comparison::is_matching;
use crate::help_data::HelpData;
use std::collections::HashSet;
use crate::adt_header::AdtHeader;
use crate::typing;
use crate::type_checker::match_types;
use crate::unification_map::UnificationMap;
use crate::type_graph::TypeGraph;
use crate::type_comparison::reduce_type;
use crate::TypeError;
use crate::help_message::ErrorMsg;
use std::iter::Rev;

#[derive(Debug, Clone, PartialEq)]
pub enum CompileMode {
    Header,
    Body,
    Module
}

#[derive(Debug, Clone)]
pub struct Context {
   function_list: String,
   pub compile_mode: CompileMode,
   pub environment: Environment,
   pub typing_context: VarType,
   kinds: Vec<(Type, Kind)>,
   nominals: TypeNominal,
   pub subtypes: TypeGraph,
   pub adt: AdtHeader,
   pub unifications: Vec<Vec<(Type, Type)>>
}

impl Default for Context {
    fn default() -> Self {
        Context { 
            function_list: include_str!("../configs/src/functions.txt").to_string(), 
            compile_mode: CompileMode::Body,
            environment: Environment::StandAlone,
            typing_context: VarType::new(),
            kinds: vec![],
            nominals: TypeNominal::new(),
            subtypes: TypeGraph::new(),
            adt: AdtHeader::default(),
            unifications: vec![]
        }
    }
}

impl From<Vec<(Lang, Type)>> for  Context {
   fn from(val: Vec<(Lang, Type)>) -> Self {
       let val2: Vec<(Var, Type)> = val.iter()
           .map(|(lan, typ)| { 
                (Var::from_language(lan.clone()).unwrap(), typ.clone())})
           .collect();
        Context { 
            typing_context: val2.into(),
            ..Context::default()
        }
   } 
}

//main
impl Context {
    pub fn new(types: Vec<(Var, Type)>, kinds: Vec<(Type, Kind)>) -> Context {
        Context {
            typing_context: types.into(),
            kinds: kinds,
            ..Context::default()
        }
    }

    pub fn get_type_from_variable(&self, var: &Var) -> Option<Type> {
        self.variables().flat_map(|(var2, type_)| {
            let Var(name1, path1, perm1, bo1, typ1, _h1) = var;
            let Var(name2, path2, perm2, bo2, typ2, _h2) = var2;
            let conditions = (name1 == name2) &&
                (path1 == path2) && (perm1 == perm2) &&
                (bo1 == bo2) && (type_comparison::is_matching(self, typ1, typ2));
            if conditions { Some(type_.clone()) } else { None }
        }).next()
    }

    pub fn get_type_from_aliases(&self, var: &Var) -> Option<Type> {
        self.aliases().flat_map(|(var2, type_)| {
            let Var(name1, path1, perm1, bo1, typ1, _h1) = var;
            let Var(name2, path2, perm2, bo2, typ2, _h2) = var2;
            let conditions = (name1 == name2) &&
                (path1 == path2) && (perm1 == perm2) &&
                (bo1 == bo2) && (type_comparison::is_matching(self, typ1, typ2));
            if conditions { Some(type_.clone()) } else { None }
        }).next()
    }

    fn is_matching(&self, var1: &Var, var2: &Var) -> bool {
        let Var(name1, path1, perm1, _bo1, params1, _h1) = var1;
        let Var(name2, path2, perm2, _bo2, params2, _h2) = var2;
        (name1 == name2) &&
            (path1 == path2) && (perm1 == perm2) &&
            (type_comparison::is_matching(self, params1, params2))
    }

    pub fn get_matching_alias_signature(&self, var: &Var) -> Option<(Type, Vec<Type>)> {
        self.aliases().find(|(var2, _)| self.is_matching(var, var2))
            .map(|(var2, target_type)| {
                if var2.is_opaque() {
                    (var2.clone().to_alias(), vec![])
                } else {
                    if let Type::Params(types, _) = var2.get_type() {
                        (target_type.clone(), types.clone())
                    } else { panic!("The related type is not Params([...])"); }
                }
            })
    }

    pub fn variables(&self) -> Rev<std::slice::Iter<'_, (Var, Type)>> {
        self.typing_context.variables()
    }

    pub fn aliases(&self) -> Rev<std::slice::Iter<'_, (Var, Type)>> {
        self.typing_context.aliases()
    }

    pub fn push_var_type(self, lang: Var, typ: Type, context: &Context) -> Context {
        let types = typ.type_extraction();
        let var_type = self.typing_context.clone().push_var_type(vec![(lang, typ.clone())]);
        let type_list: Vec<_> = var_type.get_types().iter().cloned().collect();
        let nominals = types.iter()
            .fold(self.nominals.clone(), |nom, typ_| nom.push_type(typ_.clone()));
        let mut typ_hie = self.subtypes.clone();
        typ_hie.update(&type_list);
        Context {
            typing_context: var_type, 
            nominals: nominals.clone(),
            subtypes: typ_hie,
            adt: self.clone().add_generic_function(&wasm_types(&types, &nominals, context)).adt,
            ..self
        }
    }

    pub fn get_type_from_existing_variable(&self, var: Var) -> Type {
        if let Type::RFunction(_) = var.get_type() {
            var.get_type()
        } else {
            self.typing_context.variables()
               .find(|(v, _)| var.match_with(v, self))
               .map(|(_, ty)| ty)
               .expect(&TypeError::UndefinedVariable(var.to_language()).display())
               .clone()
        }
    }

    pub fn get_true_variable(&self, var: &Var) -> Var {
        let res = self.typing_context.variables()
           .find(|(v, _)| var.match_with(v, self))
           .map(|(v, _)| v);
        match res {
            Some(vari) => vari.clone(),
            _ => self.is_an_untyped_function(&var.get_name()) 
                .then(|| var.clone().set_type(Type::RFunction(var.get_help_data())))
                .expect(&format!("The variable {} was not found:\n {}", var, self.display_typing_context()))
        }
    }

    pub fn is_an_untyped_function(&self, name: &str) -> bool {
        let formated_name = name.replace("__", ".");
        self.function_list.lines().any(|line| line.trim() == formated_name)
    }

    pub fn get_class(&self, t: &Type) -> String {
        self.nominals.get_nominal(t.clone()).1
    }

    pub fn get_classes(&self, t: &Type) -> Option<String> {
        let super_types = self.subtypes.clone().get_all_supertypes(t);
        let (_type_hierarchy, classes) = super_types.iter()
            .fold((self.nominals.clone(), vec![]), 
                  |acc, typ| {
                      let (noms, nom_str) = acc.0.get_nominal(typ.to_owned());
                      (noms, acc.1.iter().chain([format!("'{}'", nom_str)].iter()).cloned().collect())
                  });
        if classes.len() == 0 {
            None
        } else {
            Some(
                format!("c({})",
                    classes.iter().cloned().collect::<HashSet<_>>()
                        .iter().cloned().collect::<Vec<_>>()
                        .join(", ")
                        )
                )
        }
    }

    pub fn get_functions(&self, t: &Type) -> Vec<(Var, Type)> {
        self.typing_context.variables()
            .filter(|(var, _typ)| {
                let related_typ = var.get_type();
                related_typ != Type::Empty(HelpData::default())
                    && is_matching(self, t, &related_typ) 
            }).cloned()
            .collect()
    }

    pub fn get_embeddings(&self, t: &Type) -> (Vec<(Var, Type)>, Context) {
        match t {
            Type::Record(arg_typs, _) => {
                let new_t = t.clone().without_embeddings();
                let type_functions = arg_typs.iter()
                    .filter(|arg_typ| arg_typ.is_embedded())
                    .map(|arg_typ| arg_typ.remove_embeddings())
                    .map(|arg_typ| (arg_typ.get_argument(), arg_typ.get_type()))
                    .flat_map(|(arg, ty)| {
                        let funcs = self.get_functions(&ty);
                        funcs.iter().map(|(var, fun)| (arg.get_label(), var.clone(), fun.clone())).collect::<Vec<_>>()
                    })
                    .map(|(arg, var, fun): (String, Var, Type)| 
                         (arg, var.clone().set_type(new_t.clone()),
                         fun.clone().replace_function_types(var.get_type(), new_t.clone())))
                    .collect::<Vec<_>>();
                let new_cont = 
                    type_functions.iter()
                    .fold(self.clone(), |ctx, tf| ctx.clone().push_var_type(tf.1.clone(), tf.2.clone(), &ctx));
                let new_cont2 = new_cont.clone().add_generic_function(&self.build_concret_functions(&type_functions));
                (type_functions.iter().map(|(_arg, var, fun)| (var.clone(), fun.clone())).collect(),
                new_cont2)
            },
            _ => (vec![], self.clone())
        }
    }

    pub fn in_black_list(s: &str) -> bool {
        let black_list: HashSet<&str> = [
            "seq", "append", "add", "mul", "map", "dot", "t", "print"
        ].iter().cloned().collect();
        black_list.contains(s)
    }

    pub fn is_gen_func_allowed(l: &Lang) -> bool {
        if let Lang::GenFunc(_s, name, _) = l {
           if Self::in_black_list(&name) { false } else { true }
        } else { false }
    }

    pub fn add_generic_function(self, data: &[Lang]) -> Context {
        let data = data.iter()
            .filter(|x| Self::is_gen_func_allowed(x))
            .collect::<Vec<_>>();
        let adt_header = self.adt.set_generic_methods(
            data.iter()
                .fold(self.adt.generic_methods.clone(),
                |adt, lang| add_if_absent(adt, (*lang).clone().clone())));
        Context {
            adt: adt_header,
            ..self
        }
    }

    pub fn add_module_declarations(self, data: &[Lang]) -> Context {
        let adt_header = self.adt.set_module(
            data.iter()
                .fold(self.adt.modules.clone(),
                |adt, lang| add_if_absent(adt, lang.clone())));
        Context {
            adt: adt_header,
            ..self
        }
    }

    fn build_concret_functions(&self, var_typ: &[(String, Var, Type)]) -> Vec<Lang> {
        var_typ.iter().map(|(par, var, typ)| {
            let t = var.get_type();
            match typ {
                Type::Function(kinds, args, t2, h) => {
                   let manips = args.iter().enumerate()
                       .map(|(i, argtyp)| manip(&generate_arg(i), argtyp.clone(), t.clone(), par))
                       .collect::<Vec<_>>();
                   let t_end = (**t2).clone();
                   let manip1 = if t_end == t { Manip::Set(par.to_string()) } else {Manip::Same("a".to_string())};
                   let new_args = args.iter()
                       .map(|ty| if *ty == t {typ.clone()} else { ty.clone() } )
                       .enumerate()
                       .map(|(i, typ)| ArgumentType::new(&generate_arg(i), &typ.clone()))
                       .collect::<Vec<_>>();
                   let new_t2 = if t_end == t { typ.clone() } else {t_end.clone()};
                   Lang::Let(
                       var.clone(),
                        Type::Empty(HelpData::default()),
                        Box::new(
                           Lang::Function(kinds.to_vec(), new_args, new_t2,
                                          Box::new(build_concret_function(&manips, manip1, var.clone())), h.clone())
                                ),
                            h.clone())
                },
                _ => todo!()
            }
        }).collect()
    }

    pub fn get_type_from_class(&self, class: &str) -> Type {
        self.nominals.get_type_from_class(class)
    }

    pub fn push_unifications(&self, unifs: Vec<(Type, Type)>) -> Context {
        let mut new_unifications = self.unifications.clone();
        new_unifications.push(unifs);
        Context {
            unifications: new_unifications,
            ..self.clone()
        }
    }

    pub fn pop_unifications(&self) -> (Option<Vec<(Type, Type)>>, Context) {
        let mut new_unifications = self.unifications.clone();
        let popped = if !new_unifications.is_empty() {
            Some(new_unifications.remove(0))
        } else {
            None
        };
        
        (popped, Context {
            unifications: new_unifications,
            ..self.clone()
        })
    }
    pub fn add_arg_types(&self, params: &[ArgumentType]) -> Context {
        let param_types = params.iter()
            .map(|arg_typ| reduce_type(self, &arg_typ.get_type()).for_var())
            .map(|typ| match typ.to_owned() {
                Type::Function(_, typs, _, _) => {
                    if typs.len() > 0 {
                        typs[0].clone()
                    } else { typ }
                },
                t => t
            })
            .collect::<Vec<_>>();
        params.into_iter()
            .zip(param_types.clone().into_iter())
            .map(|(arg_typ, par_typ)| 
                 (Var::from_name(&arg_typ.get_argument_str())
                    .set_type(par_typ), reduce_type(self, &arg_typ.get_type())))
            .fold(self.clone(), |cont, (var, typ)| cont.clone().push_var_type(var, typ, &cont))
    }

    pub fn set_environment(&self, e: Environment) -> Context {
        Context {
            environment: e,
            ..self.clone()
        }
    }

    pub fn update_classes(&self) -> Context {
        let types = self.typing_context.get_types().iter().cloned().collect::<Vec<_>>();
        let mut new_subtypes = self.subtypes.to_owned();
        new_subtypes.update(&types);
        Context {
            subtypes: new_subtypes,
            ..self.clone()
        }
    }

    pub fn set_compile_mode(self, cm: CompileMode) -> Context {
        Context {
            compile_mode: cm,
            ..self
        }
    }

    pub fn display_typing_context(&self) -> String {
       let res = self.variables()
            .chain(self.aliases())
            .map(|(var, typ)| format!("{} ==> {}", var.to_string(), typ.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("CONTEXT:\n{}", res)
    }

    pub fn error(&self, msg: String) -> String {
        format!("{}{}", msg, self.display_typing_context())
    }

    pub fn get_unification_map(&self, values: &[Lang], param_types: &[Type]) 
        -> Option<UnificationMap> {
        let res = values.iter()
            .map(|val| typing(self, val).0)
            .zip(param_types.iter())
            .flat_map(|(val_typ, par_typ)| match_types(self, &val_typ, par_typ))
            .flatten()
            .collect::<Vec<_>>();
        (res.len() > 0).then(|| UnificationMap::new(res))
    }

    pub fn push_alias(self, alias_name: String, typ: Type) -> Self {
        Context {
            nominals: self.nominals.push_alias(alias_name.to_string(), typ),
            ..self
        }
    }

    pub fn is_in_header_mode(&self) -> bool {
        self.compile_mode == CompileMode::Header 
    }

    pub fn display_nominals(&self) -> String {
        self.nominals.display_nominals()
    }

    pub fn append_function_list(&self, t: &str) -> Context {
        Context {
            function_list: self.function_list.clone() + t,
            ..self.clone()
        } 
    }

}

fn build_concret_function(m: &[Manip], end: Manip, name: Var) -> Lang {
    let args = m.iter()
        .map(|x| x.to_lang())
        .collect::<Vec<_>>();
    match end {
        Manip::Set(param) => {
            Lang::FunctionApp(
                Box::new(Var::from_name("set").to_language()),
                vec![
                    Var::from_name("a").to_language(),
                    Lang::Char(param.to_string(), HelpData::default()),
                    Lang::FunctionApp(Box::new(name.to_language()), args.clone(), args.clone().into()),
                ],
                args.into()
            )
        },
        _ => {
            Lang::FunctionApp(Box::new(name.to_language()), args.clone(), args.into())
        }
    }
}


fn manip(arg: &str, t1: Type, t2: Type, par: &str) -> Manip {
   if t1 == t2 {
       Manip::Get(arg.to_string(), par.to_string())
   } else {
        Manip::Same(arg.to_string())
   }
}

type ArgName = String;
type Field = String;

enum Manip {
    Get(ArgName, Field),
    Set(Field),
    Same(ArgName)
}

impl Manip {
    fn to_lang(&self) -> Lang {
        match self {
            Manip::Same(argname) => Var::from_name(&argname).to_language(),
            Manip::Get(argname, field) => {
                Lang::FunctionApp(
                    Box::new(Var::from_name("get").to_language()),
                    vec![
                        Var::from_name(&argname).to_language(),
                        Lang::Char(field.to_string(), HelpData::default())
                    ], HelpData::default())
            },
            _ => todo!()
        }
    }
}

pub fn generate_arg(num: usize) -> String {
    match num {
        0 => "a",
        1 => "b",
        2 => "c",
        3 => "d",
        4 => "e",
        5 => "f",
        6 => "g",
        7 => "h",
        8 => "i",
        9 => "j",
        10 => "k",
        _ => "overflow"
    }.to_string()
}

fn add_if_absent(mut vec: Vec<Lang>, val: Lang) -> Vec<Lang> {
    if !vec.contains(&val) {
        vec.push(val);
    }
    vec // Retourne le nouveau vecteur
}

fn wasm_types(types: &[Type], nominals: &TypeNominal, cont: &Context) -> Vec<Lang> {
    types.iter().flat_map(|typ| {
        let name = nominals.get_class(typ, cont);
        match typ {
            Type::Record(_, _) | Type::Tag(_, _, _) | Type::Function(_, _, _, _)
                => Some(Lang::Alias(Var::from_name(&name), vec![], typ.clone(), typ.clone().into())),
            _ => None
        }
    }).collect::<Vec<_>>()
}
