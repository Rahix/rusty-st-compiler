// Copyright (c) 2020 Ghaith Hachem and Mathias Rieder
use super::{HardwareBinding, PouIndexEntry, VariableIndexEntry, VariableType};
use crate::ast::{
    self, ArgumentProperty, AstStatement, CompilationUnit, DataType, DataTypeDeclaration,
    Implementation, Pou, PouType, SourceRange, TypeNature, UserTypeDeclaration, VariableBlock,
    VariableBlockType,
};
use crate::diagnostics::Diagnostic;
use crate::index::{ArgumentType, Index, MemberInfo};
use crate::lexer::IdProvider;
use crate::typesystem::{self, *};

pub fn visit(unit: &CompilationUnit, mut id_provider: IdProvider) -> Index {
    let mut index = Index::default();
    //Create the typesystem
    let builtins = get_builtin_types();
    for data_type in builtins {
        index.register_type(data_type);
    }

    //Create user defined datatypes
    for user_type in &unit.types {
        visit_data_type(&mut index, &mut id_provider, user_type);
    }

    //Create defined global variables
    for global_vars in &unit.global_vars {
        visit_global_var_block(&mut index, global_vars);
    }

    //Create types and variables for POUs
    for pou in &unit.units {
        visit_pou(&mut index, pou);
    }

    for implementation in &unit.implementations {
        visit_implementation(&mut index, implementation);
    }
    index
}

pub fn visit_pou(index: &mut Index, pou: &Pou) {
    let interface_name = format!("{}_interface", &pou.name);

    let mut member_names = vec![];

    //register the pou's member variables
    let mut member_varargs = None;
    let mut count = 0;
    for block in &pou.variable_blocks {
        let block_type = get_declaration_type_for(block);
        for var in &block.variables {
            let varargs = if let DataTypeDeclaration::DataTypeDefinition {
                data_type:
                    ast::DataType::VarArgs {
                        referenced_type,
                        sized,
                    },
                ..
            } = &var.data_type
            {
                let name = referenced_type
                    .as_ref()
                    .map(|it| &**it)
                    .and_then(DataTypeDeclaration::get_name)
                    .map(|it| it.to_string());
                Some(if *sized {
                    VarArgs::Sized(name)
                } else {
                    VarArgs::Unsized(name)
                })
            } else {
                None
            };

            if varargs.is_some() {
                member_varargs = varargs.clone();
            }

            member_names.push(var.name.clone());

            if let Some(var_type_name) = var.data_type.get_name() {
                let type_name = if block_type.is_by_ref() {
                    //register a pointer type for argument
                    register_byref_pointer_type_for(index, var_type_name)
                } else {
                    var_type_name.to_string()
                };
                let initial_value = index
                    .get_mut_const_expressions()
                    .maybe_add_constant_expression(
                        var.initializer.clone(),
                        type_name.as_str(),
                        Some(pou.name.clone()),
                    );

                let binding = var.address.as_ref().and_then(|it| {
                    HardwareBinding::from_statement(index, it, Some(pou.name.clone()))
                });

                index.register_member_variable(
                    MemberInfo {
                        container_name: &pou.name,
                        variable_name: &var.name,
                        variable_linkage: block_type,
                        variable_type_name: &type_name,
                        is_constant: block.constant,
                        binding,
                        varargs,
                    },
                    initial_value,
                    var.location.clone(),
                    count,
                );
                count += 1;
            }
        }
    }

    //register a function's return type as a member variable
    let return_type_name = pou
        .return_type
        .as_ref()
        .and_then(|it| it.get_name())
        .unwrap_or(VOID_TYPE);
    if pou.return_type.is_some() {
        member_names.push(pou.get_return_name().into());
        let source_location = SourceRange::new(pou.location.get_end()..pou.location.get_end());
        index.register_member_variable(
            MemberInfo {
                container_name: &pou.name,
                variable_name: pou.get_return_name(),
                variable_linkage: ArgumentType::ByVal(VariableType::Return),
                variable_type_name: return_type_name,
                is_constant: false, //return variables are not constants
                binding: None,
                varargs: None,
            },
            None,
            source_location,
            count,
        )
    }

    let has_varargs = member_varargs.is_some();
    let datatype = typesystem::DataType {
        name: pou.name.to_string(),
        initial_value: None,
        information: DataTypeInformation::Struct {
            name: interface_name,
            member_names,
            source: StructSource::Pou(pou.pou_type.clone()),
        },
        nature: TypeNature::Any,
    };

    match &pou.pou_type {
        PouType::Program => {
            index.register_program(&pou.name, &pou.location, pou.linkage);
            index.register_pou_type(datatype);
        }
        PouType::FunctionBlock => {
            let global_struct_name = crate::index::get_initializer_name(&pou.name);
            let variable = VariableIndexEntry::create_global(
                &global_struct_name,
                &global_struct_name,
                &pou.name,
                pou.location.clone(),
            )
            .set_constant(true);
            index.register_global_initializer(&global_struct_name, variable);
            index.register_pou(PouIndexEntry::create_function_block_entry(
                &pou.name,
                pou.linkage,
            ));
            index.register_pou_type(datatype);
        }
        PouType::Class => {
            let global_struct_name = crate::index::get_initializer_name(&pou.name);
            let variable = VariableIndexEntry::create_global(
                &global_struct_name,
                &global_struct_name,
                &pou.name,
                pou.location.clone(),
            )
            .set_constant(true);
            index.register_global_initializer(&global_struct_name, variable);
            index.register_pou(PouIndexEntry::create_class_entry(&pou.name, pou.linkage));
            index.register_pou_type(datatype);
        }
        PouType::Function => {
            index.register_pou(PouIndexEntry::create_function_entry(
                &pou.name,
                return_type_name,
                &pou.generics,
                pou.linkage,
                has_varargs,
            ));
            index.register_pou_type(datatype);
        }
        PouType::Method { owner_class } => {
            index.register_pou(PouIndexEntry::create_method_entry(
                &pou.name,
                return_type_name,
                owner_class,
                pou.linkage,
            ));
            index.register_pou_type(datatype);
        }
        _ => {}
    };
}

/// returns the declaration type (ByRef or ByVal) for the given VariableBlock (VAR_INPUT, VAR_OUTPUT, VAR_INOUT, etc.)
fn get_declaration_type_for(block: &VariableBlock) -> ArgumentType {
    if matches!(
        block.variable_block_type,
        VariableBlockType::InOut
            | VariableBlockType::Output
            | VariableBlockType::Input(ArgumentProperty::ByRef)
    ) {
        ArgumentType::ByRef(get_variable_type_from_block(block))
    } else {
        ArgumentType::ByVal(get_variable_type_from_block(block))
    }
}

fn visit_implementation(index: &mut Index, implementation: &Implementation) {
    let pou_type = &implementation.pou_type;
    index.register_implementation(
        &implementation.name,
        &implementation.type_name,
        pou_type.get_optional_owner_class().as_ref(),
        pou_type.into(),
        implementation.generic,
    );
    //if we are registing an action, also register a datatype for it
    if pou_type == &PouType::Action {
        let datatype = typesystem::DataType {
            name: implementation.name.to_string(),
            initial_value: None,
            information: DataTypeInformation::Alias {
                name: implementation.name.clone(),
                referenced_type: implementation.type_name.clone(),
            },
            nature: TypeNature::Derived,
        };

        index.register_pou(PouIndexEntry::create_action_entry(
            implementation.name.as_str(),
            implementation.type_name.as_str(),
            ast::LinkageType::Internal, //TODO: where do I get correct linkage from?
        ));
        index.register_pou_type(datatype);
    }
}

fn register_byref_pointer_type_for(index: &mut Index, inner_type_name: &str) -> String {
    //get unique name
    let type_name = format!("auto_pointer_to_{}", inner_type_name);

    //generate a pointertype for the variable
    index.register_type(typesystem::DataType {
        name: type_name.clone(),
        initial_value: None,
        information: DataTypeInformation::Pointer {
            name: type_name.clone(),
            inner_type_name: inner_type_name.to_string(),
            auto_deref: true,
        },
        nature: TypeNature::Any,
    });

    type_name
}

fn visit_global_var_block(index: &mut Index, block: &VariableBlock) {
    let linkage = block.linkage;
    for var in &block.variables {
        let target_type = var.data_type.get_name().unwrap_or_default();
        let initializer = index
            .get_mut_const_expressions()
            .maybe_add_constant_expression(var.initializer.clone(), target_type, None);
        let variable = VariableIndexEntry::create_global(
            &var.name,
            &var.name,
            var.data_type.get_name().expect("named variable datatype"),
            var.location.clone(),
        )
        .set_initial_value(initializer)
        .set_constant(block.constant)
        .set_linkage(linkage)
        .set_hardware_binding(
            var.address
                .as_ref()
                .and_then(|it| HardwareBinding::from_statement(index, it, None)),
        );
        index.register_global_variable(&var.name, variable);
    }
}

fn get_variable_type_from_block(block: &VariableBlock) -> VariableType {
    match block.variable_block_type {
        VariableBlockType::Local => VariableType::Local,
        VariableBlockType::Temp => VariableType::Temp,
        VariableBlockType::Input(_) => VariableType::Input,
        VariableBlockType::Output => VariableType::Output,
        VariableBlockType::Global => VariableType::Global,
        VariableBlockType::InOut => VariableType::InOut,
    }
}

fn visit_data_type(
    index: &mut Index,
    id_provider: &mut IdProvider,
    type_declaration: &UserTypeDeclaration,
) {
    let data_type = &type_declaration.data_type;
    let scope = &type_declaration.scope;
    //names should not be empty
    match data_type {
        DataType::StructType {
            name: Some(name),
            variables,
        } => {
            let struct_name = name.as_str();

            let member_names: Vec<String> =
                variables.iter().map(|it| it.name.to_string()).collect();

            let type_name = name.clone();
            let information = DataTypeInformation::Struct {
                name: type_name.clone(),
                member_names,
                source: StructSource::OriginalDeclaration,
            };

            let init = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    type_name.as_str(),
                    scope.clone(),
                );
            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: init,
                information,
                nature: TypeNature::Derived,
            });
            //Generate an initializer for the struct
            let global_struct_name = crate::index::get_initializer_name(name);
            let variable = VariableIndexEntry::create_global(
                &global_struct_name,
                &global_struct_name,
                type_name.as_str(),
                type_declaration.location.clone(),
            )
            .set_initial_value(init)
            .set_constant(true);
            index.register_global_initializer(&global_struct_name, variable);
            for (count, var) in variables.iter().enumerate() {
                if let DataTypeDeclaration::DataTypeDefinition {
                    data_type, scope, ..
                } = &var.data_type
                {
                    //first we need to handle the inner type
                    visit_data_type(
                        index,
                        id_provider,
                        &UserTypeDeclaration {
                            data_type: data_type.clone(),
                            initializer: None,
                            location: SourceRange::undefined(),
                            scope: scope.clone(),
                        },
                    )
                }

                let member_type = var.data_type.get_name().expect("named variable datatype");
                let init = index
                    .get_mut_const_expressions()
                    .maybe_add_constant_expression(
                        var.initializer.clone(),
                        member_type,
                        scope.clone(),
                    );

                let binding = var
                    .address
                    .as_ref()
                    .and_then(|it| HardwareBinding::from_statement(index, it, scope.clone()));

                index.register_member_variable(
                    MemberInfo {
                        container_name: struct_name,
                        variable_name: &var.name,
                        variable_linkage: ArgumentType::ByVal(VariableType::Input), // struct members act like VAR_INPUT in terms of visibility
                        variable_type_name: member_type,
                        is_constant: false, //struct members are not constants //TODO thats probably not true (you can define a struct in an CONST-block?!)
                        binding,
                        varargs: None,
                    },
                    init,
                    var.location.clone(),
                    count as u32,
                );
            }
        }

        DataType::EnumType {
            name: Some(name),
            elements,
            numeric_type,
            ..
        } => {
            let enum_name = name.as_str();

            let information = DataTypeInformation::Enum {
                name: enum_name.to_string(),
                elements: ast::get_enum_element_names(elements),
                referenced_type: numeric_type.clone(),
            };

            for ele in ast::flatten_expression_list(elements) {
                let element_name = ast::get_enum_element_name(ele);
                if let AstStatement::Assignment { right, .. } = ele {
                    let init = index.get_mut_const_expressions().add_constant_expression(
                        right.as_ref().clone(),
                        numeric_type.clone(),
                        scope.clone(),
                    );
                    index.register_enum_element(
                        &element_name,
                        enum_name,
                        Some(init),
                        ele.get_location(),
                    )
                } else {
                    unreachable!("the preprocessor should have provided explicit assignments for enum values")
                }
            }

            let init = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    enum_name,
                    scope.clone(),
                );
            index.register_type(typesystem::DataType {
                name: enum_name.to_string(),
                initial_value: init,
                information,
                nature: TypeNature::Int,
            });
        }

        DataType::SubRangeType {
            name: Some(name),
            referenced_type,
            bounds,
        } => {
            let information = if let Some(AstStatement::RangeStatement { start, end, .. }) = bounds
            {
                DataTypeInformation::SubRange {
                    name: name.into(),
                    referenced_type: referenced_type.into(),
                    sub_range: (*start.clone()..*end.clone()),
                }
            } else {
                DataTypeInformation::Alias {
                    name: name.into(),
                    referenced_type: referenced_type.into(),
                }
            };

            let init = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    name,
                    scope.clone(),
                );
            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: init,
                information,
                nature: TypeNature::Int,
            });
        }
        DataType::ArrayType {
            name: Some(name),
            referenced_type,
            bounds,
        } => {
            let dimensions: Result<Vec<Dimension>, Diagnostic> = bounds
                .get_as_list()
                .iter()
                .map(|it| {
                    if let AstStatement::RangeStatement { start, end, .. } = it {
                        let constants = index.get_mut_const_expressions();
                        Ok(Dimension {
                            start_offset: TypeSize::from_expression(
                                constants.add_constant_expression(
                                    *start.clone(),
                                    typesystem::INT_TYPE.to_string(),
                                    scope.clone(),
                                ),
                            ),
                            end_offset: TypeSize::from_expression(
                                constants.add_constant_expression(
                                    *end.clone(),
                                    typesystem::INT_TYPE.to_string(),
                                    scope.clone(),
                                ),
                            ),
                        })
                    } else {
                        Err(Diagnostic::codegen_error(
                            "Invalid array definition: RangeStatement expected",
                            it.get_location(),
                        ))
                    }
                })
                .collect();
            let dimensions = dimensions.unwrap(); //TODO hmm we need to talk about all this unwrapping :-/
            let referenced_type_name = referenced_type.get_name().expect("named datatype");
            let information = DataTypeInformation::Array {
                name: name.clone(),
                inner_type_name: referenced_type_name.to_string(),
                dimensions,
            };

            let init1 = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    name,
                    scope.clone(),
                );
            // TODO unfortunately we cannot share const-expressions between multiple
            // index-entries
            let init2 = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    name,
                    scope.clone(),
                );

            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: init1,
                information,
                nature: TypeNature::Any,
            });
            let global_init_name = crate::index::get_initializer_name(name);
            if init2.is_some() {
                let variable = VariableIndexEntry::create_global(
                    global_init_name.as_str(),
                    global_init_name.as_str(),
                    name,
                    type_declaration.location.clone(),
                )
                .set_constant(true)
                .set_initial_value(init2);
                index.register_global_initializer(&global_init_name, variable);
            }
        }
        DataType::PointerType {
            name: Some(name),
            referenced_type,
            ..
        } => {
            let inner_type_name = referenced_type.get_name().expect("named datatype");
            let information = DataTypeInformation::Pointer {
                name: name.clone(),
                inner_type_name: inner_type_name.into(),
                auto_deref: false,
            };

            let init = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    name,
                    scope.clone(),
                );
            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: init,
                information,
                nature: TypeNature::Any,
            });
        }
        DataType::StringType {
            name: Some(name),
            size,
            is_wide,
            ..
        } => {
            let type_name = name;
            let encoding = if *is_wide {
                StringEncoding::Utf16
            } else {
                StringEncoding::Utf8
            };

            let size = match size {
                Some(AstStatement::LiteralInteger { value, .. }) => {
                    TypeSize::from_literal((value + 1) as i64)
                }
                Some(statement) => {
                    // construct a "x + 1" expression because we need one additional character for \0 terminator
                    let len_plus_1 = AstStatement::BinaryExpression {
                        id: statement.get_id(),
                        left: Box::new(statement.clone()),
                        operator: ast::Operator::Plus,
                        right: Box::new(AstStatement::LiteralInteger {
                            id: statement.get_id(),
                            location: statement.get_location(),
                            value: 1,
                        }),
                    };

                    TypeSize::from_expression(
                        index.get_mut_const_expressions().add_constant_expression(
                            len_plus_1,
                            type_name.clone(),
                            scope.clone(),
                        ),
                    )
                }
                None => TypeSize::from_literal((DEFAULT_STRING_LEN + 1).into()),
            };
            let information = DataTypeInformation::String { size, encoding };
            let init = index
                .get_mut_const_expressions()
                .maybe_add_constant_expression(
                    type_declaration.initializer.clone(),
                    type_name,
                    scope.clone(),
                );
            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: init,
                information,
                nature: TypeNature::String,
            });

            if init.is_some() {
                // register a global variable with the initial value to memcopy from
                let global_init_name = crate::index::get_initializer_name(name);
                let initializer_global = VariableIndexEntry::create_global(
                    global_init_name.as_str(),
                    global_init_name.as_str(),
                    name,
                    type_declaration.location.clone(),
                )
                .set_constant(true)
                .set_initial_value(init);
                index.register_global_initializer(global_init_name.as_str(), initializer_global);
            }
        }
        DataType::VarArgs { .. } => {} //Varargs are not indexed,
        DataType::GenericType {
            name,
            generic_symbol,
            nature,
        } => {
            let information = DataTypeInformation::Generic {
                name: name.clone(),
                generic_symbol: generic_symbol.clone(),
                nature: *nature,
            };
            index.register_type(typesystem::DataType {
                name: name.to_string(),
                initial_value: None,
                information,
                nature: TypeNature::Any,
            });
        }

        _ => { /* unnamed datatypes are ignored */ }
    };
}
