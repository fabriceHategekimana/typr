use crate::argument_kind::ArgumentKind;
use crate::Type;
use crate::help_data::HelpData;
use crate::builder;

#[derive(Debug)]
pub struct FunctionType(pub Vec<ArgumentKind>, pub Vec<Type>, pub Type, pub HelpData);

impl FunctionType {
    pub fn get_param_types(&self) -> Vec<Type> {
        self.1.clone()
    }

    pub fn get_ret_type(&self) -> Type {
        self.2.clone()
    }

    pub fn is_r_function(&self) -> bool {
        (self.0 == vec![]) &&
        (self.1 == vec![]) &&
        (self.2 == builder::empty_type())
    }
}

impl TryFrom<Type> for FunctionType {
    type Error = String;

    fn try_from(value: Type) -> Result<Self, Self::Error> {
        if let Type::Function(kinds, args, ret, h) = value {
            Ok(FunctionType(kinds, args, *ret, h))
        } else { 
            Err(format!("{} is a type not convertible to FunctionType", value))
        }
    }
}
