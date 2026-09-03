//! `render` 集成测试。

use harness_prompt::{
    AgentModelMessage, AgentModelRole, PromptInput, RenderError, VariableContext, expand_variables,
    render,
};

#[test]
fn render_expands_variables_in_system_and_text_parts() {
    let mut input = PromptInput::new()
        .with_system("you are {{char}}")
        .push_message(AgentModelMessage::text(AgentModelRole::User, "hi {{user}}"));
    input.character_name = Some("Alice".into());
    input.user_name = Some("Bob".into());

    let request = render(&input).expect("render");
    assert_eq!(request.system.as_deref(), Some("you are Alice"));
    assert_eq!(request.messages[0].text_payload(), "hi Bob");
}

#[test]
fn render_with_empty_input_errors() {
    assert_eq!(render(&PromptInput::new()), Err(RenderError::Empty));
}

#[test]
fn expand_variables_preserves_unknown_macros() {
    let ctx = VariableContext {
        character_name: "Alice".into(),
        user_name: "Bob".into(),
    };
    // 时间宏、随机宏等保持原样
    assert_eq!(
        expand_variables("{{time::UTC+2}} / {{roll 1d6}}", &ctx),
        "{{time::UTC+2}} / {{roll 1d6}}"
    );
}

#[test]
fn expand_variables_handles_empty_context() {
    let ctx = VariableContext::default();
    assert_eq!(expand_variables("{{char}} meets {{user}}", &ctx), " meets ");
}
