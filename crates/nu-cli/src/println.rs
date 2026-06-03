use nu_engine::command_prelude::*;
use regex::Regex;

#[derive(Clone)]
pub struct Println;

impl Command for Println {
    fn name(&self) -> &str {
        "println!"
    }

    fn signature(&self) -> Signature {
        Signature::build("println!")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("template", SyntaxShape::String, "The template string to format.")
            .rest("rest", SyntaxShape::Any, "Positional arguments to replace {}.")
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Print formatting strings like Rust's println! macro."
    }

    fn extra_description(&self) -> &str {
        r#"Supports three formatting styles simultaneously:
1. Positional placeholders: (println! "Hello {}" "world")
2. Named variables from scope: (println! "Hello {user}")
3. Dynamic expressions evaluated on the fly: (println! "2 + 2 = (2 + 2)")

Always appends a newline at the end of the output."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // 1. Pega a string base e os argumentos variádicos
        let template: String = call.req(engine_state, stack, 0)?;
        let positional_args: Vec<Value> = call.rest(engine_state, stack, 1)?;

        let mut output = template;
        let config = engine_state.config();

        // --- FASE 1: Resolver expressões dinâmicas entre parênteses (ex: (versao * 2)) ---
        let re_eval = Regex::new(r"\(([^)]+)\)").unwrap();
        let mut updated_output_eval = output.clone();
        
        for cap in re_eval.captures_iter(&output) {
            let full_match = &cap[0]; 
            let expr_text = &cap[1];  

            // Executa a expressão usando a engine nativa do Nu
            if let Ok(pipeline_result) = nu_engine::eval_string(
                engine_state,
                stack,
                expr_text,
                call.head,
                PipelineData::empty(),
            ) {
                if let Ok(value) = pipeline_result.into_value(call.head) {
                    let val_str = value.into_string("", config);
                    updated_output_eval = updated_output_eval.replace(full_match, &val_str);
                }
            }
        }
        output = updated_output_eval;

        // --- FASE 2: Resolver argumentos nomeados direto no escopo (ex: {user}) ---
        let re_named = Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
        let mut updated_output_named = output.clone();
        
        for cap in re_named.captures_iter(&output) {
            let full_match = &cap[0]; 
            let var_name = &cap[1];   

            // Varre a Stack de variáveis locais buscando pelo identificador literal
            if let Some(var_id) = engine_state.find_variable(var_name.as_bytes()) {
                if let Ok(value) = stack.get_var(var_id, call.head) {
                    let val_str = value.into_string("", config);
                    updated_output_named = updated_output_named.replace(full_match, &val_str);
                }
            }
        }
        output = updated_output_named;

        // --- FASE 3: Resolver argumentos posicionais sequenciais (ex: {}) ---
        for arg in positional_args {
            let val_str = arg.into_string("", config);
            output = output.replacen("{}", &val_str, 1);
        }

        // 2. Cospe direto no stdout nativo com quebra de linha
        println!("{}", output);

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Print a positional formatted string",
                example: r#"(println! "Hello, {}!" "world")"#,
                result: None,
            },
            Example {
                description: "Print using a variable from scope",
                example: r#"let name = "Vini"; (println! "Welcome, {name}")"#,
                result: None,
            },
        ]
    }
}

