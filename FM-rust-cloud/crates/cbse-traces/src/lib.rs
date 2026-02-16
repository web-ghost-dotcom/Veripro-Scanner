// SPDX-License-Identifier: AGPL-3.0

//! Trace rendering and visualization

use colored::*;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

/// Address type
pub type Address = u64;

/// Trace event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEvent {
    Log,
    Sload,
    Sstore,
}

/// Event log entry
#[derive(Debug, Clone)]
pub struct EventLog {
    pub address: Address,
    pub topics: Vec<Vec<u8>>,
    pub data: Vec<u8>,
}

impl EventLog {
    pub fn new(address: Address, topics: Vec<Vec<u8>>, data: Vec<u8>) -> Self {
        Self {
            address,
            topics,
            data,
        }
    }
}

/// Storage read operation
#[derive(Debug, Clone)]
pub struct StorageRead {
    pub slot: Address,
    pub value: Vec<u8>,
    pub transient: bool,
}

/// Storage write operation
#[derive(Debug, Clone)]
pub struct StorageWrite {
    pub slot: Address,
    pub value: Vec<u8>,
    pub transient: bool,
}

/// Call message
#[derive(Debug, Clone)]
pub struct CallMessage {
    pub target: Address,
    pub caller: Address,
    pub value: u64,
    pub data: Vec<u8>,
    pub call_scheme: u8,
    pub is_static: bool,
}

impl CallMessage {
    pub fn new(
        target: Address,
        caller: Address,
        value: u64,
        data: Vec<u8>,
        call_scheme: u8,
        is_static: bool,
    ) -> Self {
        Self {
            target,
            caller,
            value,
            data,
            call_scheme,
            is_static,
        }
    }

    pub fn is_create(&self) -> bool {
        self.call_scheme == 0xF0 || self.call_scheme == 0xF5 // CREATE or CREATE2
    }
}

/// Call output
#[derive(Debug, Clone)]
pub struct CallOutput {
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
    pub return_scheme: Option<u8>,
}

impl CallOutput {
    pub fn new(data: Option<Vec<u8>>, error: Option<String>, return_scheme: Option<u8>) -> Self {
        Self {
            data,
            error,
            return_scheme,
        }
    }
}

/// Trace element (can be a call context, event log, storage read, or storage write)
#[derive(Debug, Clone)]
pub enum TraceElement {
    Call(CallContext),
    Log(EventLog),
    Read(StorageRead),
    Write(StorageWrite),
}

/// Call context with trace information
#[derive(Debug, Clone)]
pub struct CallContext {
    pub message: CallMessage,
    pub output: CallOutput,
    pub depth: usize,
    pub trace: Vec<TraceElement>,
}

impl CallContext {
    pub fn new(message: CallMessage, output: CallOutput, depth: usize) -> Self {
        Self {
            message,
            output,
            depth,
            trace: Vec::new(),
        }
    }

    pub fn is_stuck(&self) -> bool {
        self.output.data.is_none() && self.output.error.is_none()
    }

    pub fn add_trace_element(&mut self, element: TraceElement) {
        self.trace.push(element);
    }
}

/// Call sequence
pub type CallSequence = Vec<CallContext>;

/// Deployment address mapper
pub struct DeployAddressMapper {
    contracts: HashMap<String, String>,
}

impl DeployAddressMapper {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    pub fn add_deployed_contract(&mut self, address: String, contract_name: String) {
        self.contracts.insert(address, contract_name);
    }

    pub fn get_deployed_contract(&self, address: &str) -> String {
        self.contracts
            .get(address)
            .cloned()
            .unwrap_or_else(|| address.to_string())
    }
}

impl Default for DeployAddressMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Get mnemonic for opcode
pub fn mnemonic(opcode: u8) -> &'static str {
    match opcode {
        0xF0 => "CREATE",
        0xF1 => "CALL",
        0xF2 => "CALLCODE",
        0xF3 => "RETURN",
        0xF4 => "DELEGATECALL",
        0xF5 => "CREATE2",
        0xFA => "STATICCALL",
        0xFD => "REVERT",
        0xFE => "INVALID",
        0xFF => "SELFDESTRUCT",
        0x54 => "SLOAD",
        0x55 => "SSTORE",
        0x5C => "TLOAD",
        0x5D => "TSTORE",
        _ => "UNKNOWN",
    }
}

/// Convert bytes to hex string
pub fn hexify(data: &[u8]) -> String {
    if data.is_empty() {
        return "0x".to_string();
    }
    format!("0x{}", hex::encode(data))
}

/// Get byte length of data
pub fn byte_length(data: &[u8]) -> usize {
    data.len()
}

/// Render address with optional contract name replacement
pub fn rendered_address(addr: Address, mapper: &DeployAddressMapper) -> String {
    let addr_str = format!("0x{:x}", addr);
    mapper.get_deployed_contract(&addr_str)
}

/// Render slot value
pub fn rendered_slot(slot: Address) -> String {
    if slot < 0x10000 {
        format!("{}", slot).magenta().to_string()
    } else {
        format!("0x{:x}", slot).magenta().to_string()
    }
}

/// Render event log
pub fn rendered_log(log: &EventLog) -> String {
    let opcode_str = format!("LOG{}", log.topics.len());
    let mut parts = Vec::new();

    for (i, topic) in log.topics.iter().enumerate() {
        parts.push(format!(
            "{}={}",
            format!("topic{}", i).cyan(),
            hexify(topic)
        ));
    }
    parts.push(format!("{}={}", "data".cyan(), hexify(&log.data)));

    format!("{}({})", opcode_str, parts.join(", "))
}

/// Render storage write
pub fn rendered_sstore(update: &StorageWrite) -> String {
    let slot_str = rendered_slot(update.slot);
    let opcode = if update.transient { "TSTORE" } else { "SSTORE" };
    format!(
        "{} @{} ← {}",
        opcode.cyan(),
        slot_str,
        hexify(&update.value)
    )
}

/// Render storage read
pub fn rendered_sload(read: &StorageRead) -> String {
    let slot_str = rendered_slot(read.slot);
    let opcode = if read.transient { "TLOAD" } else { "SLOAD" };
    format!("{} @{} → {}", opcode.cyan(), slot_str, hexify(&read.value))
}

/// Render calldata
pub fn rendered_calldata(calldata: &[u8], contract_name: Option<&str>) -> String {
    if calldata.is_empty() {
        return "0x".to_string();
    }

    if calldata.len() < 4 {
        return hexify(calldata);
    }

    if calldata.len() == 4 {
        let selector = hexify(&calldata[..4]);
        return format!("{}()", selector);
    }

    let selector = &calldata[..4];
    let args = &calldata[4..];
    format!("{}({})", hexify(selector), hexify(args))
}

/// Render initcode for CREATE calls
pub fn rendered_initcode(context: &CallContext) -> String {
    let data = &context.message.data;
    let initcode_str = hexify(data);
    format!("{}({})", initcode_str, "".cyan())
}

/// Render call output
pub fn render_output(context: &CallContext, writer: &mut dyn Write) -> io::Result<()> {
    let output = &context.output;
    let failed = output.error.is_some();

    if !failed && context.is_stuck() {
        return Ok(());
    }

    let returndata_str = if let Some(ref data) = output.data {
        let is_create = context.message.is_create();
        if is_create && !failed {
            format!("<{} bytes of code>", byte_length(data))
        } else {
            hexify(data)
        }
    } else {
        "0x".to_string()
    };

    let ret_scheme_str = if let Some(ret_scheme) = output.return_scheme {
        format!("{} ", mnemonic(ret_scheme).cyan())
    } else {
        String::new()
    };

    let error_str = if let Some(ref error) = output.error {
        format!(" (error: {:?})", error)
    } else {
        String::new()
    };

    let indent = "    ".repeat(context.depth);
    let symbol = if failed { "↩ ".red() } else { "↩ ".green() };
    let data_colored = if failed {
        returndata_str.red()
    } else {
        returndata_str.green()
    };
    let error_colored = error_str.red();

    writeln!(
        writer,
        "{}{}{}{}{}",
        indent, symbol, ret_scheme_str, data_colored, error_colored
    )
}

/// Render trace recursively
pub fn render_trace(
    context: &CallContext,
    mapper: &DeployAddressMapper,
    trace_events: &[TraceEvent],
    writer: &mut dyn Write,
) -> io::Result<()> {
    let message = &context.message;
    let addr_str = rendered_address(message.target, mapper);
    let caller_str = format!(" (caller: {})", rendered_address(message.caller, mapper));

    let value_str = if message.value > 0 {
        format!(" (value: {})", message.value)
    } else {
        String::new()
    };

    let call_scheme_str = format!("{} ", mnemonic(message.call_scheme).cyan());
    let indent = "    ".repeat(context.depth);

    if message.is_create() {
        let initcode_str = format!("<{} bytes of initcode>", byte_length(&message.data));
        writeln!(
            writer,
            "{}{}{}{}{}",
            indent, call_scheme_str, addr_str, initcode_str, value_str
        )?;
    } else {
        let calldata = rendered_calldata(&message.data, Some(&addr_str));
        let call_str = format!("{}::{}", addr_str, calldata);
        let static_str = if message.is_static {
            " [static]".yellow()
        } else {
            ColoredString::from("")
        };
        writeln!(
            writer,
            "{}{}{}{}{}{}",
            indent, call_scheme_str, call_str, static_str, value_str, caller_str
        )?;
    }

    let log_indent = "    ".repeat(context.depth + 1);
    for trace_element in &context.trace {
        match trace_element {
            TraceElement::Call(call_ctx) => {
                render_trace(call_ctx, mapper, trace_events, writer)?;
            }
            TraceElement::Log(event_log) => {
                if trace_events.contains(&TraceEvent::Log) {
                    writeln!(writer, "{}{}", log_indent, rendered_log(event_log))?;
                }
            }
            TraceElement::Read(storage_read) => {
                if trace_events.contains(&TraceEvent::Sload) {
                    writeln!(writer, "{}{}", log_indent, rendered_sload(storage_read))?;
                }
            }
            TraceElement::Write(storage_write) => {
                if trace_events.contains(&TraceEvent::Sstore) {
                    writeln!(writer, "{}{}", log_indent, rendered_sstore(storage_write))?;
                }
            }
        }
    }

    render_output(context, writer)?;

    if context.depth == 1 {
        write!(writer, "")?;
    }

    Ok(())
}

/// Render call sequence
pub fn render_call_sequence(
    call_sequence: &CallSequence,
    mapper: &DeployAddressMapper,
    trace_events: &[TraceEvent],
    writer: &mut dyn Write,
) -> io::Result<()> {
    for call in call_sequence {
        render_trace(call, mapper, trace_events, writer)?;
    }
    Ok(())
}

/// Get rendered trace as string
pub fn rendered_trace(
    context: &CallContext,
    mapper: &DeployAddressMapper,
    trace_events: &[TraceEvent],
) -> String {
    let mut buffer = Vec::new();
    render_trace(context, mapper, trace_events, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Get rendered call sequence as string
pub fn rendered_call_sequence(
    call_sequence: &CallSequence,
    mapper: &DeployAddressMapper,
    trace_events: &[TraceEvent],
) -> String {
    let mut buffer = Vec::new();
    render_call_sequence(call_sequence, mapper, trace_events, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic() {
        assert_eq!(mnemonic(0xF0), "CREATE");
        assert_eq!(mnemonic(0xF1), "CALL");
        assert_eq!(mnemonic(0xF3), "RETURN");
        assert_eq!(mnemonic(0xFD), "REVERT");
        assert_eq!(mnemonic(0x54), "SLOAD");
        assert_eq!(mnemonic(0x55), "SSTORE");
    }

    #[test]
    fn test_hexify() {
        assert_eq!(hexify(&[]), "0x");
        assert_eq!(hexify(&[0x12, 0x34]), "0x1234");
        assert_eq!(hexify(&[0xFF, 0xAB]), "0xffab");
    }

    #[test]
    fn test_byte_length() {
        assert_eq!(byte_length(&[]), 0);
        assert_eq!(byte_length(&[1, 2, 3]), 3);
        assert_eq!(byte_length(&[0; 100]), 100);
    }

    #[test]
    fn test_call_message_is_create() {
        let create_msg = CallMessage::new(0, 0, 0, vec![], 0xF0, false);
        assert!(create_msg.is_create());

        let call_msg = CallMessage::new(0, 0, 0, vec![], 0xF1, false);
        assert!(!call_msg.is_create());
    }

    #[test]
    fn test_deploy_address_mapper() {
        let mut mapper = DeployAddressMapper::new();
        mapper.add_deployed_contract("0x123".to_string(), "MyContract".to_string());

        assert_eq!(mapper.get_deployed_contract("0x123"), "MyContract");
        assert_eq!(mapper.get_deployed_contract("0x456"), "0x456");
    }

    #[test]
    fn test_rendered_address() {
        let mut mapper = DeployAddressMapper::new();
        mapper.add_deployed_contract("0x123".to_string(), "TestContract".to_string());

        assert_eq!(rendered_address(0x123, &mapper), "TestContract");
        assert_eq!(rendered_address(0x456, &mapper), "0x456");
    }

    #[test]
    fn test_rendered_slot_small() {
        let slot = rendered_slot(42);
        assert!(slot.contains("42"));
    }

    #[test]
    fn test_rendered_slot_large() {
        let slot = rendered_slot(0x123456);
        assert!(slot.contains("0x123456"));
    }

    #[test]
    fn test_rendered_calldata_empty() {
        assert_eq!(rendered_calldata(&[], None), "0x");
    }

    #[test]
    fn test_rendered_calldata_short() {
        assert_eq!(rendered_calldata(&[0x12], None), "0x12");
    }

    #[test]
    fn test_rendered_calldata_selector_only() {
        let result = rendered_calldata(&[0x12, 0x34, 0x56, 0x78], None);
        assert!(result.contains("0x12345678"));
        assert!(result.contains("()"));
    }

    #[test]
    fn test_rendered_calldata_with_args() {
        let data = vec![0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD];
        let result = rendered_calldata(&data, None);
        assert!(result.contains("0x12345678"));
        assert!(result.contains("0xabcd"));
    }

    #[test]
    fn test_event_log() {
        let address = 0x1234567890abcdefu64;
        let log = EventLog {
            address,
            topics: vec![vec![0x12, 0x34], vec![0x56, 0x78]],
            data: vec![0xAB, 0xCD],
        };
        let rendered = rendered_log(&log);
        assert!(rendered.contains("LOG2"));
        assert!(rendered.contains("topic0"));
        assert!(rendered.contains("topic1"));
        assert!(rendered.contains("data"));
    }

    #[test]
    fn test_storage_read() {
        let read = StorageRead {
            slot: 42,
            value: vec![0x12, 0x34],
            transient: false,
        };
        let rendered = rendered_sload(&read);
        assert!(rendered.contains("SLOAD"));
        assert!(rendered.contains("42"));
        assert!(rendered.contains("0x1234"));
    }

    #[test]
    fn test_storage_read_transient() {
        let read = StorageRead {
            slot: 42,
            value: vec![0x12, 0x34],
            transient: true,
        };
        let rendered = rendered_sload(&read);
        assert!(rendered.contains("TLOAD"));
    }

    #[test]
    fn test_storage_write() {
        let write = StorageWrite {
            slot: 10,
            value: vec![0xFF, 0xEE],
            transient: false,
        };
        let rendered = rendered_sstore(&write);
        assert!(rendered.contains("SSTORE"));
        assert!(rendered.contains("10"));
        assert!(rendered.contains("0xffee"));
    }

    #[test]
    fn test_storage_write_transient() {
        let write = StorageWrite {
            slot: 10,
            value: vec![0xFF, 0xEE],
            transient: true,
        };
        let rendered = rendered_sstore(&write);
        assert!(rendered.contains("TSTORE"));
    }

    #[test]
    fn test_call_context_is_stuck() {
        let msg = CallMessage::new(0, 0, 0, vec![], 0xF1, false);
        let output = CallOutput::new(None, None, None);
        let ctx = CallContext::new(msg, output, 1);
        assert!(ctx.is_stuck());
    }

    #[test]
    fn test_call_context_not_stuck_with_data() {
        let msg = CallMessage::new(0, 0, 0, vec![], 0xF1, false);
        let output = CallOutput::new(Some(vec![0x12]), None, None);
        let ctx = CallContext::new(msg, output, 1);
        assert!(!ctx.is_stuck());
    }

    #[test]
    fn test_call_context_not_stuck_with_error() {
        let msg = CallMessage::new(0, 0, 0, vec![], 0xF1, false);
        let output = CallOutput::new(None, Some("error".to_string()), None);
        let ctx = CallContext::new(msg, output, 1);
        assert!(!ctx.is_stuck());
    }

    #[test]
    fn test_call_context_add_trace_element() {
        let msg = CallMessage::new(0, 0, 0, vec![], 0xF1, false);
        let output = CallOutput::new(None, None, None);
        let mut ctx = CallContext::new(msg, output, 1);

        assert_eq!(ctx.trace.len(), 0);

        let address = 0x1234567890abcdefu64;
        let log = EventLog {
            address,
            topics: vec![],
            data: vec![],
        };
        ctx.add_trace_element(TraceElement::Log(log));

        assert_eq!(ctx.trace.len(), 1);
    }
}
