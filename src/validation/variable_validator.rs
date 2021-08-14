use crate::{
    ast::{DataType, DataTypeDeclaration, SourceRange, Variable, VariableBlock},
    index::Index,
    Diagnostic,
};

use super::ValidationContext;

/// validates variables & datatypes

pub struct VariableValidator<'i> {
    index: &'i Index,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'i> VariableValidator<'i> {
    pub fn new(index: &'i Index) -> VariableValidator {
        VariableValidator { index, diagnostics: Vec::new() }
    }

    pub fn validate_variable_block(&self, _block: &VariableBlock) {}

    pub fn validate_variable(&self, _variable: &Variable) {}

    pub fn validate_data_type_declaration(
        &self,
        _declaration: &DataTypeDeclaration,
    ) {
    }

    pub fn validate_data_type(
        &mut self,
        declaration: &DataType,
        location: &SourceRange
    ) {
        match declaration {
            DataType::StructType { variables, .. } => {
                if variables.is_empty() {
                    self.diagnostics.push(Diagnostic::empty_variable_block(location.clone()));
                }
            }
            DataType::EnumType { elements, .. } => {
                if elements.is_empty() {
                    self.diagnostics.push(Diagnostic::empty_variable_block(location.clone()));
                }
            }
            DataType::SubRangeType {
                name,
                referenced_type,
                bounds,
            } => {},
            DataType::ArrayType {
                name,
                bounds,
                referenced_type,
            } => {},
            DataType::StringType {
                name,
                is_wide,
                size,
            } => {},
            DataType::VarArgs { referenced_type } => {},
        }
    }
}

#[cfg(test)]
mod variable_validator_tests {
    use crate::{validation::validation_tests::parse_and_validate, Diagnostic};

    #[test]
    fn validate_empty_struct_declaration() {
        let diagnostics = parse_and_validate(
            "
        TYPE the_struct : STRUCT END_STRUCT END_TYPE
            
        PROGRAM prg
            VAR
                my_struct : STRUCT
                END_STRUCT
            END_VAR
        END_PROGRAM
        ",
        );

        assert_eq!(
            diagnostics,
            vec![
                Diagnostic::empty_variable_block((14..44).into()),
                Diagnostic::empty_variable_block((131..164).into())
            ]
        );
    }

    #[test]
    fn validate_empty_enum_declaration() {
        let diagnostics = parse_and_validate(
            "
        TYPE my_enum : (); END_TYPE
            
        PROGRAM prg
            VAR
                my_enum : ();
            END_VAR
        END_PROGRAM
        ",
        );

        assert_eq!(
            diagnostics,
            vec![
                Diagnostic::empty_variable_block((14..27).into()),
                Diagnostic::empty_variable_block((112..114).into())
            ]
        );
    }
}
