//! `render` / `expand_variables` 集成测试。
//!
//! ponytail: 单元测试全部下沉到 tests/ —— Cargo 对 tests/ 目录默认只在
//! `cargo test` 编译，写 `#[cfg(test)]` 在这里反而是 boilerplate 错误。

use harness_prompt::{
    AgentModelContentPart, AgentModelMessage, AgentModelRole, PromptInput, RenderError,
    VariableContext, expand_variables, render,
};

// ===== render =====

#[test]
fn renders_system_and_messages_in_order() {
    let input = PromptInput::new()
        .with_system("you are {{char}}")
        .push_message(AgentModelMessage::text(AgentModelRole::User, "hi {{user}}"));
    let mut ctx_input = input;
    ctx_input.character_name = Some("Alice".into());
    ctx_input.user_name = Some("Bob".into());
    let request = render(&ctx_input).expect("render");
    assert_eq!(request.system.as_deref(), Some("you are Alice"));
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].text_payload(), "hi Bob");
}

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
fn rejects_fully_empty_input() {
    let input = PromptInput::new();
    assert_eq!(render(&input), Err(RenderError::Empty));
}

#[test]
fn render_with_empty_input_errors() {
    assert_eq!(render(&PromptInput::new()), Err(RenderError::Empty));
}

#[test]
fn empty_input_with_only_system_is_ok() {
    let input = PromptInput::new().with_system("sys");
    let request = render(&input).expect("render");
    assert!(request.messages.is_empty());
    assert_eq!(request.system.as_deref(), Some("sys"));
}

#[test]
fn text_part_gets_variable_expansion() {
    let mut msg = AgentModelMessage::text(AgentModelRole::User, "hi {{char}}");
    msg.parts.push(AgentModelContentPart::Text {
        text: "second {{user}}".into(),
    });
    let input = PromptInput {
        character_name: Some("Alice".into()),
        user_name: Some("Bob".into()),
        messages: vec![msg],
        ..PromptInput::new()
    };
    let request = render(&input).expect("render");
    assert_eq!(request.messages[0].parts.len(), 2);
    assert_eq!(
        request.messages[0].parts[0],
        AgentModelContentPart::Text {
            text: "hi Alice".into()
        }
    );
    assert_eq!(
        request.messages[0].parts[1],
        AgentModelContentPart::Text {
            text: "second Bob".into()
        }
    );
}

// ===== expand_variables =====

#[test]
fn expand_variables_preserves_unknown_macros() {
    let ctx = VariableContext {
        character_name: "Alice".into(),
        user_name: "Bob".into(),
    };
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
