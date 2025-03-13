mod language;
mod elements;
mod parser;
mod types;
mod operators;
mod my_io;
mod var;
mod metaprogramming;
mod argument_type;
mod argument_value;
mod argument_kind;
mod context_manager;
mod type_comparison;
mod unification;
mod context;
mod adt;
mod module;
mod nominal_context;
mod r#type;
mod kinds;
mod kind;
mod type_checker;
mod type_context;
mod type_module;
mod type_printer;
mod tag;
mod index;
mod adt_manager;
mod nominals;

use parser::parse;
use my_io::{read_file, execute_r};
use crate::r#type::Type;
use crate::language::Lang;
use crate::metaprogramming::metaprogrammation;
use std::fs::File;
use std::io::Write;
use crate::adt::Adt;
use crate::type_checker::typing;
use crate::context::Context;
use crate::nominal_context::NominalContext;
use crate::adt_manager::AdtManager;

fn write_adt_to_r(adt: &Adt, nominal: &NominalContext, cont: &Context) -> () {
    let rstd = include_str!("../configs/std.R");
    let mut app = File::create("app.R").unwrap();
    let content = format!("{}\n\n{}", rstd, adt.to_r(nominal, cont));
    app.write_all(content.as_bytes()).unwrap();
}

fn execute(adt: &Adt, nominal: NominalContext, cont: &Context) -> () {
    write_adt_to_r(&adt, &nominal, cont);
    execute_r();
}

fn type_check(adt: &Adt) -> (NominalContext, Context) {
    let (typ, context) = typing(&Context::default(), &Lang::Sequence(adt.0.clone()));
    type_printer::pretty_print(&typ);
    (NominalContext::from(&context), context)
}

fn parse_code() -> AdtManager {
    let typr_std = include_str!("../configs/std.ty");
    let adt_manager = AdtManager::new()
        .add_to_body(parse(&read_file()).unwrap().1)
        .add_to_header(parse(typr_std).unwrap().1);
    let adt = metaprogrammation(adt_manager.body.clone());
    adt_manager.set_body(adt)
}

fn main() {
    let adt_manager = parse_code();

    let (nominal, context) = type_check(&adt_manager.get_adt_with_header());
    execute(&adt_manager.get_adt_without_header(), nominal, &context);
}

#[cfg(test)]
mod tests {
    use crate::elements::parse_elements;
    use crate::language::Lang;

    #[test]
    fn test1(){
        let text = "true and false and true and true;";
        let value = parse_elements(text).unwrap().1;
        assert_eq!(
            value,
            Lang::Empty);
    }

    #[test]
    fn test2() {
        let text = "true or false;";
        let value = parse_elements(text).unwrap().1;
        assert_eq!(
            value, 
            Lang::Empty);
    }
}
