use crate::my_io::read_file_from_name;
use crate::argument_type::ArgumentType;
use crate::var::Permission;
use crate::adt::Adt;
use crate::var::Var;
use crate::Lang;
use crate::Type;
use crate::parse;
use crate::module::Module;

fn get_alias(t: Type) -> Type {
    Type::Alias(t.clone().get_name(), vec![], "empty".to_string())
}

fn find_module(name: &str, seq: &[Lang]) -> Module {
    Module::from_language(seq.iter().find(|line| {
        match line {
            Lang::Module(mod_name, _) 
                if mod_name == name => true,
            _ => false
        }
    }).unwrap().clone()).unwrap()
}

fn get_core_function(var: Var, seq: &[Lang]) -> Lang {
    if var.get_path() == "" {
        seq.iter().find(|x| match x { 
                    Lang::Let(v, _, _) 
                        if v.get_name() == var.get_name()
                        => true,
                    _ => false
                    }).expect("Module type embedding: can't find the corresponding variable").clone()
    } else {
        let module = find_module(&var.get_path(), seq);
        get_core_function(var.set_path(""), &module.get_body())
    }
}

fn implement(func: &Lang, name: String, parent_type: &Type, field: String, child_type: &Type, adt: &Adt) -> Lang {
    let parent_type2 = get_alias(parent_type.clone());
    match func {
        Lang::Function(kinds, types, typ, body) => {
            let types2: &mut Vec<ArgumentType> = &mut types.clone();
            if types.len() > 0{
                types2[0].1 = parent_type2.clone();
            }
            let mut typ2 = typ;
            let val = parent_type2.clone();
            if typ == child_type {
               typ2 =  &val;
            }
            // {field: a.field.fname()} | a
            let func = Lang::Function(kinds.to_vec(), types2.clone(), typ2.clone(), body.clone());
            Lang::Let(Var(
                    name,
                    "".to_string(),
                    Permission::Private,
                    false,
                    parent_type2
                    ),
                    Type::Any,
                    Box::new(func))
        },
        Lang::Variable(s1, s2, perm, mutop, typ) => {
            let var = Var::from_language(Lang::Variable(s1.clone(), s2.clone(), perm.clone(), mutop.clone(), typ.clone())).unwrap();
            let fun = get_core_function(var, &adt.0);
            implement(&fun, name, parent_type, field, child_type, adt)
        },
        element => element.clone()
    }
}

fn get_related_functions(typ: Type, adt: &Adt) -> Vec<(Lang, String)> {
    adt.0.iter().flat_map(|line| {
        match line {
            Lang::Let(Var( name, _path, _permission, _mut, typ2), _typ3, elem) 
                if *typ2 == typ
                => Some((*elem.clone(), name.clone()))
                ,
            _ => None 
        }
    }).collect::<Vec<_>>()
}


fn get_record_embeddings(name: Type, args: Vec<ArgumentType>) -> Vec<(Type, ArgumentType)> {
    args.iter().filter(|ArgumentType(_, _, emb)| *emb).map(|x| (name.clone(), x.clone())).collect()
}

fn get_embeddings(line: &Lang, adt: &Adt) -> Vec<(Type, ArgumentType)> {
    match line {
        Lang::Alias(_var, _params, Type::Tag(_name2, typ))
            => {
                match *typ.clone() {
                    Type::Embedded(ty) => vec![(Type::Empty, ArgumentType("0".to_string(), *ty, true))],
                    _ => vec![]
                }
            }
        Lang::Alias(var, params, Type::Record(ve))
            => {
                get_record_embeddings(
                    Type::Alias(var.clone().get_name(), params.clone(), Type::Record(ve.to_vec()).to_string()),
                    ve.to_vec())
        }
        Lang::Alias(_, _, Type::Alias(name, _, path)) 
            => {
                let (alias, module) = adt.find_alias_module(path, name);
                get_embeddings(&alias, &Adt(module.get_body()))

            },
        _ => vec![]
    }
}

fn type_embedding(adt: Adt) -> Adt {
    adt.iter().flat_map(|line| {
        let res = get_embeddings(line, &adt);
        if res.len() > 0 {
            res.iter().flat_map(|(parent_type, ArgumentType(field, typ, _emb))| {
                    let impls = get_related_functions(typ.clone(), &adt)
                        .iter().map(|(func, name)| implement(func, name.to_string(), &parent_type, field.to_string(), &typ, &adt))
                        .collect::<Vec<_>>(); 
                    [line.clone()].iter().chain(impls.iter()).cloned().collect::<Vec<_>>()
            }).collect::<Vec<_>>()
        } else {vec![line.clone()]}
    }).collect::<Vec<_>>().into()
}

fn import_file_module_code(line: &Lang) -> Lang {
    match line {
        Lang::ModImp(name) => {
            let new_adt = metaprogrammation(parse(&read_file_from_name(&name)).unwrap().1);
            Lang::Module(name.to_string(), new_adt.0)
        }
        n => n.clone()
    }
}

fn import_file_modules_code(adt: Adt) -> Adt {
    adt.iter().map(import_file_module_code).collect::<Vec<_>>().into()
}

fn get_related_functions2(module_name: &str, name: &str, adt: &Adt) -> Vec<Lang> {
    adt.0.iter().flat_map(|line| {
        match line {
            Lang::Let(Var(name1, _, permission, muta, Type::Alias(name2, args, path2)), typ3, _elem) 
                if name == name2 &&  *permission == Permission::Public
                    => {
                    let typ2 =  Type::Alias(name2.clone(), args.clone(), path2.clone());
                    let var1 = Lang::Variable(name1.clone(), module_name.to_string(), permission.clone(), muta.clone(), typ2.clone());
                    let var2 = Var(name1.clone(), "".to_string(), permission.clone(), muta.clone(), typ2.clone());
                    Some(Lang::Let(var2, typ3.clone(), Box::new(var1)))
                    },
            _ => None 
        }
    }).collect::<Vec<_>>()
}


fn get_related_module_functions(name: &str, typ: Type, adt: &Adt) -> Vec<Lang> {
    adt.0.iter().flat_map(|line| {
        match line {
            Lang::Module(name1, body) 
                if name1 == name
                    => {
                        let alias_name = match typ.clone() {
                            Type::Alias(na, _, _) => na,
                            _ => "".to_string()
                        };
                        get_related_functions2(name, &alias_name, &Adt(body.to_vec()))
                    }
            _ => vec![]
        }
    }).collect::<Vec<_>>()
}

fn remove_imports(lets: &[Lang]) -> Vec<Lang> {
    lets.iter().flat_map(|line| {
        match line {
            Lang::Import(_) => None,
            l => Some(l.clone())
        }
    }).collect::<Vec<_>>()
}

fn import_types(adt: Adt) -> Adt {
    let res = adt.0.iter().flat_map(|line| {
        match line {
            Lang::Import(Type::Alias(name, params, path)) => {
                let typ = Type::Alias(name.to_string(), params.clone(), path.to_string());
                let mut lets = get_related_module_functions(&path, typ.clone(), &adt);
                lets.insert(0, 
                            Lang::Alias(Var::from_name(name).set_type(Type::Params(params.clone())),
                            params.to_vec(), typ));
                remove_imports(&lets)
            },
            l => vec![l.clone()]
        }
    }).collect::<Vec<_>>();
    res.into()
}

fn private_public_change(module_name: &str, adt: Adt) -> Vec<Lang> {
    adt.0.iter().map(|line| {
        match line {
            Lang::Let(var, typ, body) 
                => Lang::Let(
                    var.clone().add_path(module_name),
                    typ.clone(), body.clone()),
            Lang::Alias(var, params, typ) 
                => Lang::Alias(
                    var.clone().add_path(module_name),
                    params.clone(), typ.clone()),
            _ => Lang::Empty
        }
    }).collect::<Vec<_>>()
}

fn unnest_module(line: &Lang) -> Vec<Lang> {
    match line {
        Lang::Module(name, body) 
            => {
                let new_adt = unnest_modules(body.clone().into());
                let mut lines = private_public_change(name, new_adt);
                lines.insert(0, Lang::ModuleDecl(name.to_string()));
                lines
            },
        lang => vec![lang.clone()]
    }
}

fn unnest_modules(adt: Adt) -> Adt {
    adt.iter().flat_map(unnest_module).collect::<Vec<_>>().into()
}

pub fn metaprogrammation(adt: Adt) -> Adt {
    //type_embedding(import_types(import_modules(adt)))
    unnest_modules(import_file_modules_code(adt))
}
