#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! TXFirewall packet policy primitives.

/// IP address representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpAddress {
    /// IPv4 address.
    V4([u8; 4]),
    /// IPv6 address.
    V6([u8; 16]),
}

/// Transport protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportProtocol {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
    /// ICMP or ICMPv6.
    Icmp,
    /// Other protocol number.
    Other(u8),
}

/// Packet metadata evaluated by firewall policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketMeta {
    /// Source address.
    pub src: IpAddress,
    /// Destination address.
    pub dst: IpAddress,
    /// Transport protocol.
    pub protocol: TransportProtocol,
    /// Source port when applicable.
    pub src_port: Option<u16>,
    /// Destination port when applicable.
    pub dst_port: Option<u16>,
    /// Packet length in bytes.
    pub len: usize,
}

/// Firewall decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    /// Allow the packet.
    Allow,
    /// Drop the packet.
    Deny,
}

/// A simple match rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rule {
    /// Optional protocol match.
    pub protocol: Option<TransportProtocol>,
    /// Optional destination port match.
    pub dst_port: Option<u16>,
    /// Action returned on match.
    pub action: Action,
}

impl Rule {
    /// Returns true when the rule matches packet metadata.
    pub fn matches(&self, packet: &PacketMeta) -> bool {
        if let Some(protocol) = self.protocol {
            if !transport_eq(protocol, packet.protocol) {
                return false;
            }
        }

        if let Some(dst_port) = self.dst_port {
            if packet.dst_port != Some(dst_port) {
                return false;
            }
        }

        true
    }
}

const fn transport_eq(left: TransportProtocol, right: TransportProtocol) -> bool {
    match (left, right) {
        (TransportProtocol::Tcp, TransportProtocol::Tcp)
        | (TransportProtocol::Udp, TransportProtocol::Udp)
        | (TransportProtocol::Icmp, TransportProtocol::Icmp) => true,
        (TransportProtocol::Other(a), TransportProtocol::Other(b)) => a == b,
        _ => false,
    }
}

/// Firewall rule table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Firewall<const MAX_RULES: usize> {
    rules: [Option<Rule>; MAX_RULES],
    default_action: Action,
}

impl<const MAX_RULES: usize> Firewall<MAX_RULES> {
    /// Creates a firewall that denies by default.
    pub const fn deny_by_default() -> Self {
        Self {
            rules: [None; MAX_RULES],
            default_action: Action::Deny,
        }
    }

    /// Adds a rule to the first free slot.
    pub fn add_rule(&mut self, rule: Rule) -> Result<(), RuleError> {
        for slot in &mut self.rules {
            if slot.is_none() {
                *slot = Some(rule);
                return Ok(());
            }
        }

        Err(RuleError::Full)
    }

    /// Evaluates packet metadata.
    pub fn evaluate(&self, packet: &PacketMeta) -> Action {
        for rule in self.rules.iter().flatten() {
            if rule.matches(packet) {
                return rule.action;
            }
        }

        self.default_action
    }
}

/// Rule table errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleError {
    /// Rule table is full.
    Full,
}

/// Token-bucket rate limiter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateLimiter {
    capacity: u32,
    available: u32,
    refill_per_tick: u32,
}

impl RateLimiter {
    /// Creates a rate limiter.
    pub const fn new(capacity: u32, refill_per_tick: u32) -> Self {
        Self {
            capacity,
            available: capacity,
            refill_per_tick,
        }
    }

    /// Refills tokens for a number of ticks.
    pub fn refill(&mut self, ticks: u32) {
        let refill = self.refill_per_tick.saturating_mul(ticks);
        self.available = self.capacity.min(self.available.saturating_add(refill));
    }

    /// Consumes one token when available.
    pub fn allow_one(&mut self) -> bool {
        if self.available == 0 {
            return false;
        }

        self.available -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firewall_denies_by_default_and_allows_matching_rule() {
        let mut firewall = Firewall::<4>::deny_by_default();
        let packet = PacketMeta {
            src: IpAddress::V4([10, 0, 0, 1]),
            dst: IpAddress::V4([10, 0, 0, 2]),
            protocol: TransportProtocol::Tcp,
            src_port: Some(50_000),
            dst_port: Some(443),
            len: 128,
        };

        assert_eq!(firewall.evaluate(&packet), Action::Deny);
        firewall
            .add_rule(Rule {
                protocol: Some(TransportProtocol::Tcp),
                dst_port: Some(443),
                action: Action::Allow,
            })
            .unwrap();
        assert_eq!(firewall.evaluate(&packet), Action::Allow);
    }

    #[test]
    fn rate_limiter_enforces_capacity() {
        let mut limiter = RateLimiter::new(2, 1);
        assert!(limiter.allow_one());
        assert!(limiter.allow_one());
        assert!(!limiter.allow_one());
        limiter.refill(1);
        assert!(limiter.allow_one());
    }
}
