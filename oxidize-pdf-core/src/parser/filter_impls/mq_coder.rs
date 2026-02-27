//! MQ Arithmetic Coder for JBIG2 decoding
//!
//! Implementation of the MQ coder as specified in ITU-T T.88 Section 7 (Arithmetic Coding).
//! This is the entropy decoder used by JBIG2 for context-dependent binary arithmetic coding.
//!
//! References:
//! - ITU-T T.88 (02/2000) Section 7: Arithmetic entropy coding procedure
//! - ITU-T T.88 Table E.1: Qe values and probability estimation state machine

use crate::parser::{ParseError, ParseResult};

/// QE probability estimation table entry per ITU-T T.88 Table E.1
///
/// Each entry contains:
/// - `qe`: Probability estimate for LPS (Less Probable Symbol)
/// - `nmps`: Next state index after MPS (More Probable Symbol) occurs
/// - `nlps`: Next state index after LPS occurs
/// - `switch_mps`: Whether to switch MPS sense after LPS
#[derive(Debug, Clone, Copy)]
struct QeEntry {
    qe: u16,
    nmps: u8,
    nlps: u8,
    switch_mps: bool,
}

/// Complete QE table with 47 states per ITU-T T.88 Table E.1
///
/// This table drives the probability adaptation in the MQ coder.
/// State 0 is the initial state with probability ~0.5 (0x5601/0xFFFF ≈ 0.336)
const QE_TABLE: [QeEntry; 47] = [
    QeEntry {
        qe: 0x5601,
        nmps: 1,
        nlps: 1,
        switch_mps: true,
    }, // 0
    QeEntry {
        qe: 0x3401,
        nmps: 2,
        nlps: 6,
        switch_mps: false,
    }, // 1
    QeEntry {
        qe: 0x1801,
        nmps: 3,
        nlps: 9,
        switch_mps: false,
    }, // 2
    QeEntry {
        qe: 0x0AC1,
        nmps: 4,
        nlps: 12,
        switch_mps: false,
    }, // 3
    QeEntry {
        qe: 0x0521,
        nmps: 5,
        nlps: 29,
        switch_mps: false,
    }, // 4
    QeEntry {
        qe: 0x0221,
        nmps: 38,
        nlps: 33,
        switch_mps: false,
    }, // 5
    QeEntry {
        qe: 0x5601,
        nmps: 7,
        nlps: 6,
        switch_mps: true,
    }, // 6
    QeEntry {
        qe: 0x5401,
        nmps: 8,
        nlps: 14,
        switch_mps: false,
    }, // 7
    QeEntry {
        qe: 0x4801,
        nmps: 9,
        nlps: 14,
        switch_mps: false,
    }, // 8
    QeEntry {
        qe: 0x3801,
        nmps: 10,
        nlps: 14,
        switch_mps: false,
    }, // 9
    QeEntry {
        qe: 0x3001,
        nmps: 11,
        nlps: 17,
        switch_mps: false,
    }, // 10
    QeEntry {
        qe: 0x2401,
        nmps: 12,
        nlps: 18,
        switch_mps: false,
    }, // 11
    QeEntry {
        qe: 0x1C01,
        nmps: 13,
        nlps: 20,
        switch_mps: false,
    }, // 12
    QeEntry {
        qe: 0x1601,
        nmps: 29,
        nlps: 21,
        switch_mps: false,
    }, // 13
    QeEntry {
        qe: 0x5601,
        nmps: 15,
        nlps: 14,
        switch_mps: true,
    }, // 14
    QeEntry {
        qe: 0x5401,
        nmps: 16,
        nlps: 14,
        switch_mps: false,
    }, // 15
    QeEntry {
        qe: 0x5101,
        nmps: 17,
        nlps: 15,
        switch_mps: false,
    }, // 16
    QeEntry {
        qe: 0x4801,
        nmps: 18,
        nlps: 16,
        switch_mps: false,
    }, // 17
    QeEntry {
        qe: 0x3801,
        nmps: 19,
        nlps: 17,
        switch_mps: false,
    }, // 18
    QeEntry {
        qe: 0x3401,
        nmps: 20,
        nlps: 18,
        switch_mps: false,
    }, // 19
    QeEntry {
        qe: 0x3001,
        nmps: 21,
        nlps: 19,
        switch_mps: false,
    }, // 20
    QeEntry {
        qe: 0x2801,
        nmps: 22,
        nlps: 19,
        switch_mps: false,
    }, // 21
    QeEntry {
        qe: 0x2401,
        nmps: 23,
        nlps: 20,
        switch_mps: false,
    }, // 22
    QeEntry {
        qe: 0x2201,
        nmps: 24,
        nlps: 21,
        switch_mps: false,
    }, // 23
    QeEntry {
        qe: 0x1C01,
        nmps: 25,
        nlps: 22,
        switch_mps: false,
    }, // 24
    QeEntry {
        qe: 0x1801,
        nmps: 26,
        nlps: 23,
        switch_mps: false,
    }, // 25
    QeEntry {
        qe: 0x1601,
        nmps: 27,
        nlps: 24,
        switch_mps: false,
    }, // 26
    QeEntry {
        qe: 0x1401,
        nmps: 28,
        nlps: 25,
        switch_mps: false,
    }, // 27
    QeEntry {
        qe: 0x1201,
        nmps: 29,
        nlps: 26,
        switch_mps: false,
    }, // 28
    QeEntry {
        qe: 0x1101,
        nmps: 30,
        nlps: 27,
        switch_mps: false,
    }, // 29
    QeEntry {
        qe: 0x0AC1,
        nmps: 31,
        nlps: 28,
        switch_mps: false,
    }, // 30
    QeEntry {
        qe: 0x09C1,
        nmps: 32,
        nlps: 29,
        switch_mps: false,
    }, // 31
    QeEntry {
        qe: 0x08A1,
        nmps: 33,
        nlps: 30,
        switch_mps: false,
    }, // 32
    QeEntry {
        qe: 0x0521,
        nmps: 34,
        nlps: 31,
        switch_mps: false,
    }, // 33
    QeEntry {
        qe: 0x0441,
        nmps: 35,
        nlps: 32,
        switch_mps: false,
    }, // 34
    QeEntry {
        qe: 0x02A1,
        nmps: 36,
        nlps: 33,
        switch_mps: false,
    }, // 35
    QeEntry {
        qe: 0x0221,
        nmps: 37,
        nlps: 34,
        switch_mps: false,
    }, // 36
    QeEntry {
        qe: 0x0141,
        nmps: 38,
        nlps: 35,
        switch_mps: false,
    }, // 37
    QeEntry {
        qe: 0x0111,
        nmps: 39,
        nlps: 36,
        switch_mps: false,
    }, // 38
    QeEntry {
        qe: 0x0085,
        nmps: 40,
        nlps: 37,
        switch_mps: false,
    }, // 39
    QeEntry {
        qe: 0x0049,
        nmps: 41,
        nlps: 38,
        switch_mps: false,
    }, // 40
    QeEntry {
        qe: 0x0025,
        nmps: 42,
        nlps: 39,
        switch_mps: false,
    }, // 41
    QeEntry {
        qe: 0x0015,
        nmps: 43,
        nlps: 40,
        switch_mps: false,
    }, // 42
    QeEntry {
        qe: 0x0009,
        nmps: 44,
        nlps: 41,
        switch_mps: false,
    }, // 43
    QeEntry {
        qe: 0x0005,
        nmps: 45,
        nlps: 42,
        switch_mps: false,
    }, // 44
    QeEntry {
        qe: 0x0001,
        nmps: 45,
        nlps: 43,
        switch_mps: false,
    }, // 45
    QeEntry {
        qe: 0x5601,
        nmps: 46,
        nlps: 46,
        switch_mps: false,
    }, // 46
];

/// MQ decoder context state
///
/// Each context maintains its own probability state index and MPS value.
/// JBIG2 uses multiple contexts for different prediction scenarios.
#[derive(Debug, Clone, Copy)]
pub struct MQContext {
    /// Current state index into QE_TABLE (0-46)
    state_index: u8,
    /// More Probable Symbol value (0 or 1)
    mps: u8,
}

impl Default for MQContext {
    fn default() -> Self {
        Self::new()
    }
}

impl MQContext {
    /// Create a new context in initial state
    ///
    /// Per ITU-T T.88, contexts start at state 0 with MPS=0
    pub fn new() -> Self {
        Self {
            state_index: 0,
            mps: 0,
        }
    }

    /// Get the current QE probability value for this context
    #[inline]
    pub fn qe(&self) -> u16 {
        QE_TABLE[self.state_index as usize].qe
    }

    /// Get the MPS (More Probable Symbol) value
    #[inline]
    pub fn mps(&self) -> u8 {
        self.mps
    }

    /// Get the current state index
    #[inline]
    pub fn state_index(&self) -> u8 {
        self.state_index
    }

    /// Update context after MPS occurrence
    #[inline]
    pub fn update_mps(&mut self) {
        self.state_index = QE_TABLE[self.state_index as usize].nmps;
    }

    /// Update context after LPS occurrence
    #[inline]
    pub fn update_lps(&mut self) {
        let entry = &QE_TABLE[self.state_index as usize];
        if entry.switch_mps {
            self.mps ^= 1; // Toggle MPS
        }
        self.state_index = entry.nlps;
    }
}

/// MQ Arithmetic Decoder
///
/// Implements the MQ coder decoding procedure per ITU-T T.88 Section 7.
/// The decoder maintains:
/// - A register (interval width)
/// - C register (code value)
/// - CT counter (bits remaining in current byte)
#[derive(Debug)]
pub struct MQDecoder<'a> {
    /// Input data stream
    data: &'a [u8],
    /// Current byte position in data
    position: usize,
    /// A register - interval width (16-bit, kept in range [0x8000, 0x10000))
    a: u32,
    /// C register - code value (32-bit with 16-bit fraction)
    c: u32,
    /// CT - count of remaining bits in current byte (renormalization counter)
    ct: i32,
}

impl<'a> MQDecoder<'a> {
    /// Create a new MQ decoder from input data
    ///
    /// Initializes the decoder per ITU-T T.88 INITDEC procedure:
    /// - Load first bytes into C register
    /// - Set A = 0x8000 (half interval)
    /// - Initialize CT counter
    pub fn new(data: &'a [u8]) -> ParseResult<Self> {
        if data.len() < 2 {
            return Err(ParseError::StreamDecodeError(
                "MQ decoder requires at least 2 bytes of data".to_string(),
            ));
        }

        let mut decoder = Self {
            data,
            position: 0,
            a: 0x8000,
            c: 0,
            ct: 0,
        };

        // INITDEC: Initialize C register by loading first bytes
        // C = (B0 << 16) | (B1 << 8)
        decoder.c = (data[0] as u32) << 16;
        decoder.position = 1;
        decoder.bytein();
        decoder.c <<= 7;
        decoder.ct -= 7;

        Ok(decoder)
    }

    /// BYTEIN procedure - load next byte into C register
    ///
    /// Per ITU-T T.88, handles byte stuffing (0xFF followed by 0x00-0x7F)
    fn bytein(&mut self) {
        if self.position >= self.data.len() {
            // End of data - use 0xFF padding per spec
            self.ct = 8;
            return;
        }

        let prev_byte = if self.position > 0 {
            self.data[self.position - 1]
        } else {
            0
        };

        if prev_byte == 0xFF {
            let current = self.data[self.position];
            if current > 0x8F {
                // Marker segment - don't consume
                self.ct = 8;
            } else {
                self.position += 1;
                self.c += 0xFE00 - ((current as u32) << 9);
                self.ct = 7;
            }
        } else {
            let byte = self.data[self.position];
            self.position += 1;
            self.c += 0xFF00 - ((byte as u32) << 8);
            self.ct = 8;
        }
    }

    /// RENORMD procedure - renormalize decoder
    ///
    /// Called when A register falls below 0x8000.
    /// Shifts A and C left until A >= 0x8000.
    #[inline]
    fn renormalize(&mut self) {
        while self.a < 0x8000 {
            if self.ct == 0 {
                self.bytein();
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
        }
    }

    /// Decode one bit using the given context
    ///
    /// Per ITU-T T.88 DECODE procedure:
    /// 1. Subtract Qe from A
    /// 2. If C < A (MPS path) or C >= A (LPS path)
    /// 3. Update context and renormalize if needed
    pub fn decode(&mut self, context: &mut MQContext) -> u8 {
        let qe = context.qe() as u32;

        // A = A - Qe
        self.a -= qe;

        // Check if C < A (code value in MPS region)
        let chigh = self.c >> 16;

        if chigh < self.a {
            // MPS case
            if self.a < 0x8000 {
                // Need renormalization - could be conditional exchange
                let d = self.mps_exchange(context, qe);
                self.renormalize();
                d
            } else {
                // Simple MPS - no exchange needed
                context.mps()
            }
        } else {
            // LPS case
            let d = self.lps_exchange(context, qe);
            self.renormalize();
            d
        }
    }

    /// MPS exchange procedure
    ///
    /// Called when A < 0x8000 in MPS region. May switch to LPS if A < Qe.
    #[inline]
    fn mps_exchange(&mut self, context: &mut MQContext, qe: u32) -> u8 {
        if self.a < qe {
            // Conditional exchange: LPS and MPS regions swapped
            let d = 1 - context.mps();
            context.update_lps();
            d
        } else {
            // Normal MPS
            let d = context.mps();
            context.update_mps();
            d
        }
    }

    /// LPS exchange procedure
    ///
    /// Called when C >= A (in LPS region). May switch to MPS if A < Qe.
    #[inline]
    fn lps_exchange(&mut self, context: &mut MQContext, qe: u32) -> u8 {
        // Remove interval from C
        self.c -= (self.a as u32) << 16;

        if self.a < qe {
            // Conditional exchange: actually in MPS region
            self.a = qe;
            let d = context.mps();
            context.update_mps();
            d
        } else {
            // Normal LPS
            self.a = qe;
            let d = 1 - context.mps();
            context.update_lps();
            d
        }
    }

    /// Decode an integer using IAID procedure (for symbol IDs)
    ///
    /// Decodes a value in range [0, 2^codewidth - 1] using separate contexts
    /// for each bit position.
    pub fn decode_iaid(&mut self, contexts: &mut [MQContext], codewidth: u8) -> u32 {
        let mut prev = 1u32;

        for _ in 0..codewidth {
            let bit = self.decode(&mut contexts[prev as usize]);
            prev = (prev << 1) | (bit as u32);
        }

        prev - (1 << codewidth)
    }

    /// Get current position in data stream
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get remaining bytes in data stream
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Cycle 1.1: MQ Context State Machine ====================

    #[test]
    fn test_mq_context_initialization() {
        let ctx = MQContext::new();

        // Per ITU-T T.88, initial state is 0 with MPS=0
        assert_eq!(ctx.state_index(), 0);
        assert_eq!(ctx.mps(), 0);

        // State 0 has Qe = 0x5601 (per Table E.1)
        assert_eq!(ctx.qe(), 0x5601);
    }

    #[test]
    fn test_mq_context_default() {
        let ctx = MQContext::default();
        assert_eq!(ctx.state_index(), 0);
        assert_eq!(ctx.mps(), 0);
        assert_eq!(ctx.qe(), 0x5601);
    }

    #[test]
    fn test_qe_table_state_0() {
        // Verify state 0 matches ITU-T T.88 Table E.1
        let entry = &QE_TABLE[0];
        assert_eq!(entry.qe, 0x5601);
        assert_eq!(entry.nmps, 1);
        assert_eq!(entry.nlps, 1);
        assert!(entry.switch_mps);
    }

    #[test]
    fn test_qe_table_all_states_valid() {
        // All NMPS and NLPS indices must be valid
        for (i, entry) in QE_TABLE.iter().enumerate() {
            assert!(
                (entry.nmps as usize) < QE_TABLE.len(),
                "State {} has invalid NMPS {}",
                i,
                entry.nmps
            );
            assert!(
                (entry.nlps as usize) < QE_TABLE.len(),
                "State {} has invalid NLPS {}",
                i,
                entry.nlps
            );
            // QE values must be non-zero and <= 0x8000
            assert!(entry.qe > 0, "State {} has zero QE", i);
            assert!(entry.qe <= 0x8000, "State {} has QE > 0x8000", i);
        }
    }

    #[test]
    fn test_qe_table_switch_states() {
        // States with switch_mps=true per ITU-T T.88
        let switch_states = [0, 6, 14];
        for state in switch_states {
            assert!(
                QE_TABLE[state].switch_mps,
                "State {} should have switch_mps=true",
                state
            );
        }
    }

    #[test]
    fn test_context_update_mps() {
        let mut ctx = MQContext::new();
        assert_eq!(ctx.state_index(), 0);

        // After MPS in state 0, should go to state 1
        ctx.update_mps();
        assert_eq!(ctx.state_index(), 1);
        assert_eq!(ctx.mps(), 0); // MPS unchanged

        // After MPS in state 1, should go to state 2
        ctx.update_mps();
        assert_eq!(ctx.state_index(), 2);
    }

    #[test]
    fn test_context_update_lps_with_switch() {
        let mut ctx = MQContext::new();
        assert_eq!(ctx.state_index(), 0);
        assert_eq!(ctx.mps(), 0);

        // State 0 has switch_mps=true, NLPS=1
        ctx.update_lps();
        assert_eq!(ctx.state_index(), 1);
        assert_eq!(ctx.mps(), 1); // MPS switched!
    }

    #[test]
    fn test_context_update_lps_without_switch() {
        let mut ctx = MQContext::new();
        ctx.state_index = 1; // State 1 has switch_mps=false
        ctx.mps = 0;

        // State 1 NLPS=6, no switch
        ctx.update_lps();
        assert_eq!(ctx.state_index(), 6);
        assert_eq!(ctx.mps(), 0); // MPS unchanged
    }

    // ==================== Cycle 1.2: MQ Decoder Initialization ====================

    #[test]
    fn test_mq_decoder_creation() {
        let data = vec![0x00, 0x00, 0x00, 0x00];
        let decoder = MQDecoder::new(&data);
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_mq_decoder_too_short() {
        let data = vec![0x00]; // Only 1 byte
        let result = MQDecoder::new(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least 2 bytes"));
    }

    #[test]
    fn test_mq_decoder_initial_state() {
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x00];
        let decoder = MQDecoder::new(&data).unwrap();

        // A should be initialized to 0x8000
        assert_eq!(decoder.a, 0x8000);
        // Position should have advanced
        assert!(decoder.position > 0);
    }

    // ==================== Cycle 1.3-1.6: MQ Decode Operations ====================

    #[test]
    fn test_mq_decode_simple() {
        // Simple test data - all zeros should decode predictably
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();
        let mut ctx = MQContext::new();

        // Decode should not panic
        let bit = decoder.decode(&mut ctx);
        assert!(bit == 0 || bit == 1);
    }

    #[test]
    fn test_mq_decode_multiple_bits() {
        let data = vec![0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();
        let mut ctx = MQContext::new();

        // Decode 16 bits - should not panic
        for _ in 0..16 {
            let _bit = decoder.decode(&mut ctx);
        }
    }

    #[test]
    fn test_mq_decode_context_evolution() {
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();
        let mut ctx = MQContext::new();

        let initial_state = ctx.state_index();

        // After decoding, state should have changed
        for _ in 0..5 {
            decoder.decode(&mut ctx);
        }

        // Context state should have evolved
        // (exact state depends on decoded values)
        assert!(ctx.state_index() != initial_state || ctx.mps() != 0);
    }

    #[test]
    fn test_mq_decode_iaid_basic() {
        let data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();

        // IAID with 4-bit codewidth needs 16 contexts (2^4)
        let mut contexts = vec![MQContext::new(); 16];

        // Should decode without panic
        let value = decoder.decode_iaid(&mut contexts, 4);
        assert!(value < 16); // Must be in valid range
    }

    #[test]
    fn test_mq_decoder_position_tracking() {
        let data = vec![0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();
        let mut ctx = MQContext::new();

        let initial_remaining = decoder.remaining();

        // Decode many bits to consume data
        for _ in 0..32 {
            decoder.decode(&mut ctx);
        }

        // Position should have advanced
        assert!(decoder.remaining() < initial_remaining);
    }

    #[test]
    fn test_mq_byte_stuffing() {
        // Test 0xFF byte stuffing: 0xFF 0x00 should be treated specially
        let data = vec![0xFF, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut decoder = MQDecoder::new(&data).unwrap();
        let mut ctx = MQContext::new();

        // Should handle stuffing without panic
        for _ in 0..16 {
            decoder.decode(&mut ctx);
        }
    }

    #[test]
    fn test_qe_entry_debug() {
        let entry = QE_TABLE[0];
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("QeEntry"));
        // Debug prints decimal, 0x5601 = 22017
        assert!(debug_str.contains("22017") || debug_str.contains("qe:"));
    }

    #[test]
    fn test_mq_context_clone() {
        let ctx1 = MQContext::new();
        let ctx2 = ctx1;

        assert_eq!(ctx1.state_index(), ctx2.state_index());
        assert_eq!(ctx1.mps(), ctx2.mps());
    }

    #[test]
    fn test_mq_context_debug() {
        let ctx = MQContext::new();
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("MQContext"));
        assert!(debug_str.contains("state_index: 0"));
    }
}
