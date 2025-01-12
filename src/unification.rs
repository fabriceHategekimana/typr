use crate::types::Type;
use crate::argument_type::ArgumentType;
use crate::context::Context;
//use crate::type_comparison::get_common_type_denominator;

pub fn type_substitution(type_: &Type, substitutions: &[(String, Type)]) -> Type {
    if substitutions.is_empty() {
        return type_.clone();
    }

    match type_ {
        // Generic type substitution
        Type::Generic(name) => {
            if let Some((_, replacement)) = substitutions.iter()
                .find(|(gen_name, _)| gen_name == name) {
                replacement.clone()
            } else {
                type_.clone()
            }
        }

        // Index generic substitution
        Type::IndexGen(name) => {
            if let Some((_, replacement)) = substitutions.iter()
                .find(|(idx_name, _)| idx_name == name) {
                replacement.clone()
            } else {
                type_.clone()
            }
        }

        // Arithmetic operations
        Type::Add(t1, t2) => {
            let v1 = type_substitution(t1, substitutions);
            let v2 = type_substitution(t2, substitutions);
            match (v1.clone(), v2.clone()) {
                (Type::Number, Type::Number) => Type::Number,
                (Type::Integer, Type::Integer) => Type::Integer,
                _ => Type::Add(Box::new(v1), Box::new(v2))
            }
        }

        Type::Minus(t1, t2) => {
            let v1 = type_substitution(t1, substitutions);
            let v2 = type_substitution(t2, substitutions);
            match (v1.clone(), v2.clone()) {
                (Type::Number, Type::Number) => Type::Number,
                (Type::Integer, Type::Integer) => Type::Integer,
                _ => Type::Minus(Box::new(v1), Box::new(v2))
            }
        }

        Type::Mul(t1, t2) => {
            let v1 = type_substitution(t1, substitutions);
            let v2 = type_substitution(t2, substitutions);
            match (v1.clone(), v2.clone()) {
                (Type::Number, Type::Number) => Type::Number,
                (Type::Integer, Type::Integer) => Type::Integer,
                _ => Type::Mul(Box::new(v1), Box::new(v2))
            }
        }

        Type::Div(t1, t2) => {
            let v1 = type_substitution(t1, substitutions);
            let v2 = type_substitution(t2, substitutions);
            match (v1.clone(), v2.clone()) {
                (Type::Number, Type::Number) => Type::Number,
                (Type::Integer, Type::Integer) => Type::Integer,
                _ => Type::Div(Box::new(v1), Box::new(v2))
            }
        }

        // Array type substitution
        Type::Array(size, element_type) => {
            Type::Array(
                Box::new(type_substitution(size, substitutions)),
                Box::new(type_substitution(element_type, substitutions))
            )
        }

        // Record type substitution
        Type::Record(fields) => {
            Type::Record(
                fields.iter()
                    .map(|arg_type| {
                        ArgumentType(
                            arg_type.0.clone(),
                            type_substitution(&arg_type.1, substitutions),
                            arg_type.2)
                    })
                    .collect()
            )
        }

        // Function type substitution
        Type::Function(params, return_type) => {
            Type::Function(
                params.iter()
                    .map(|param| type_substitution(param, substitutions))
                    .collect(),
                Box::new(type_substitution(return_type, substitutions))
            )
        }

        // Alias type substitution
        Type::Alias(name, params, base_type) => {
            Type::Alias(
                name.clone(),
                params.iter()
                    .map(|param| type_substitution(param, substitutions))
                    .collect(),
                base_type.clone()
            )
        }

        // Tag type substitution
        Type::Tag(name, inner_type) => {
            Type::Tag(
                name.clone(),
                Box::new(type_substitution(inner_type, substitutions))
            )
        }

        // Default case: return the type unchanged
        _ => type_.clone()
    }
}

// Add these new functions to the previous implementation

fn unification_helper(
    values: &[Type],
    type1: &Type,
    type2: &Type
) -> Option<Vec<(String, Type)>> {
    match (type1, type2) {
        // Direct equality case
        (t1, t2) if t1 == t2 => Some(vec![]),

        // Generic case
        (t, Type::Generic(g)) | (Type::Generic(g), t) => {
            Some(vec![(g.clone(), t.clone())])
        }

        // Index generic case with number
        (Type::Number, Type::IndexGen(g)) | (Type::IndexGen(g), Type::Number) => {
            Some(vec![(g.clone(), Type::Number)])
        }

        (Type::Integer, Type::IndexGen(g)) | (Type::IndexGen(g), Type::Integer) => {
            Some(vec![(g.clone(), Type::Integer)])
        }

        // Function case
        (Type::Function(params1, ret1), Type::Function(params2, ret2)) => {
            if params1.len() != params2.len() {
                return None;
            }

            // Unify return types
            let mut matches = unification_helper(values, ret1, ret2)?;

            // Unify parameters
            for (p1, p2) in params1.iter().zip(params2.iter()) {
                let param_matches = unification_helper(values, p1, p2)?;
                merge_substitutions(&mut matches, param_matches);
            }

            Some(matches)
        }

        // Array case
        (Type::Array(size1, elem1), Type::Array(size2, elem2)) => {
            let size_matches = unification_helper(values, size1, size2)?;
            let elem_matches = unification_helper(values, elem1, elem2)?;
            let mut combined = size_matches;
            merge_substitutions(&mut combined, elem_matches);
            Some(combined)
        }

        // Tag case
        (Type::Tag(name1, type1), Type::Tag(name2, type2)) if name1 == name2 => {
            unification_helper(values, type1, type2)
        }

        // Record case
        (Type::Record(fields1), Type::Record(fields2)) => {
            if let Some((intersection1, intersection2)) = record_intersection(fields1, fields2) {
                let types1: Vec<_> = intersection1.iter().map(|arg| &arg.1).collect();
                let types2: Vec<_> = intersection2.iter().map(|arg| &arg.1).collect();
                
                let mut all_matches = vec![];
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    if let Some(matches) = unification_helper(values, t1, t2) {
                        merge_substitutions(&mut all_matches, matches);
                    } else {
                        return None;
                    }
                }
                Some(all_matches)
            } else {
                None
            }
        }

        // Default case - types are not unifiable
        _ => None
    }
}

pub fn unify(type1: &Type, type2: &Type) -> Option<Vec<(String, Type)>> {
    // First try to get common type denominator
    //if let Some(common) = get_common_type_denominator(&Context::default(), type1, type2) {
        //// Check if the result is coherent (no contradicting substitutions)
        //if is_coherent(&common) {
            //return Some(vec![]);
        //}
    //}

    // If that doesn't work, try unification helper
    let result = unification_helper(&vec![], type1, type2)?;
    
    // Check if the result is coherent
    if is_coherent(&result) {
        Some(result)
    } else {
        None
    }
}

// Helper functions needed for unification

fn merge_substitutions(existing: &mut Vec<(String, Type)>, new: Vec<(String, Type)>) {
    for (name, type_) in new {
        if let Some(pos) = existing.iter().position(|(n, _)| n == &name) {
            existing[pos] = (name, type_);
        } else {
            existing.push((name, type_));
        }
    }
}

fn is_coherent(substitutions: &[(String, Type)]) -> bool {
    // Check for contradicting substitutions
    for i in 0..substitutions.len() {
        for j in (i + 1)..substitutions.len() {
            if substitutions[i].0 == substitutions[j].0 && 
               substitutions[i].1 != substitutions[j].1 {
                return false;
            }
        }
    }
    true
}

pub fn record_intersection(
    record1: &[ArgumentType],
    record2: &[ArgumentType]
) -> Option<(Vec<ArgumentType>, Vec<ArgumentType>)> {
    // Get labels (left elements) from both records
    let labels1: Vec<String> = record1.iter()
        .map(|arg| arg.0.clone())  // Assuming ArgumentType has a label field
        .collect();
    
    let labels2: Vec<String> = record2.iter()
        .map(|arg| arg.0.clone())
        .collect();

    // Find intersection of labels
    let common_labels: Vec<String> = labels1.iter()
        .filter(|label| labels2.contains(label))
        .cloned()
        .collect();

    // Get values for the common labels from each record
    let mut values1 = Vec::new();
    let mut values2 = Vec::new();

    for label in &common_labels {
        if let Some(value1) = record1.iter()
            .find(|arg| &arg.0 == label)
            .cloned() {
            if let Some(value2) = record2.iter()
                .find(|arg| &arg.0 == label)
                .cloned() {
                values1.push(value1);
                values2.push(value2);
            }
        }
    }

    // Merge labels with their respective values
    let intersection1 = common_labels.iter()
        .zip(values1.into_iter())
        .map(|(_label, value)| value)
        .collect();

    let intersection2 = common_labels.iter()
        .zip(values2.into_iter())
        .map(|(_label, value)| value)
        .collect();

    Some((intersection1, intersection2))
}