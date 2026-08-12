pub mod op {
    pub const NOP: u8 = 0x00;
    pub const LD_BC_D16: u8 = 0x01;
    pub const LD_BC_IND_A: u8 = 0x02;
    pub const INC_BC: u8 = 0x03;
    pub const INC_B: u8 = 0x04;
    pub const DEC_B: u8 = 0x05;
    pub const LD_B_D8: u8 = 0x06;
    pub const RLCA: u8 = 0x07;
    pub const LD_A16_IND_SP: u8 = 0x08;
    pub const ADD_HL_BC: u8 = 0x09;
    pub const LD_A_BC_IND: u8 = 0x0A;
    pub const DEC_BC: u8 = 0x0B;
    pub const INC_C: u8 = 0x0C;
    pub const DEC_C: u8 = 0x0D;
    pub const LD_C_D8: u8 = 0x0E;
    pub const RRCA: u8 = 0x0F;

    pub const STOP: u8 = 0x10;
    pub const LD_DE_D16: u8 = 0x11;
    pub const LD_DE_IND_A: u8 = 0x12;
    pub const INC_DE: u8 = 0x13;
    pub const INC_D: u8 = 0x14;
    pub const DEC_D: u8 = 0x15;
    pub const LD_D_D8: u8 = 0x16;
    pub const RLA: u8 = 0x17;
    pub const JR_R8: u8 = 0x18;
    pub const ADD_HL_DE: u8 = 0x19;
    pub const LD_A_DE_IND: u8 = 0x1A;
    pub const DEC_DE: u8 = 0x1B;
    pub const INC_E: u8 = 0x1C;
    pub const DEC_E: u8 = 0x1D;
    pub const LD_E_D8: u8 = 0x1E;
    pub const RRA: u8 = 0x1F;

    pub const JR_NZ_R8: u8 = 0x20;
    pub const LD_HL_D16: u8 = 0x21;
    pub const LD_HLI_A: u8 = 0x22;
    pub const INC_HL: u8 = 0x23;
    pub const INC_H: u8 = 0x24;
    pub const DEC_H: u8 = 0x25;
    pub const LD_H_D8: u8 = 0x26;
    pub const DAA: u8 = 0x27;
    pub const JR_Z_R8: u8 = 0x28;
    pub const ADD_HL_HL: u8 = 0x29;
    pub const LD_A_HLI: u8 = 0x2A;
    pub const DEC_HL: u8 = 0x2B;
    pub const INC_L: u8 = 0x2C;
    pub const DEC_L: u8 = 0x2D;
    pub const LD_L_D8: u8 = 0x2E;
    pub const CPL: u8 = 0x2F;

    pub const JR_NC_R8: u8 = 0x30;
    pub const LD_SP_D16: u8 = 0x31;
    pub const LD_HLD_A: u8 = 0x32;
    pub const INC_SP: u8 = 0x33;
    pub const INC_HL_IND: u8 = 0x34;
    pub const DEC_HL_IND: u8 = 0x35;
    pub const LD_HL_IND_D8: u8 = 0x36;
    pub const SCF: u8 = 0x37;
    pub const JR_C_R8: u8 = 0x38;
    pub const ADD_HL_SP: u8 = 0x39;
    pub const LD_A_HLD: u8 = 0x3A;
    pub const DEC_SP: u8 = 0x3B;
    pub const INC_A: u8 = 0x3C;
    pub const DEC_A: u8 = 0x3D;
    pub const LD_A_D8: u8 = 0x3E;
    pub const CCF: u8 = 0x3F;

    pub const LD_B_B: u8 = 0x40;
    pub const LD_B_C: u8 = 0x41;
    pub const LD_B_D: u8 = 0x42;
    pub const LD_B_E: u8 = 0x43;
    pub const LD_B_H: u8 = 0x44;
    pub const LD_B_L: u8 = 0x45;
    pub const LD_B_HL_IND: u8 = 0x46;
    pub const LD_B_A: u8 = 0x47;

    pub const LD_C_B: u8 = 0x48;
    pub const LD_C_C: u8 = 0x49;
    pub const LD_C_D: u8 = 0x4A;
    pub const LD_C_E: u8 = 0x4B;
    pub const LD_C_H: u8 = 0x4C;
    pub const LD_C_L: u8 = 0x4D;
    pub const LD_C_HL_IND: u8 = 0x4E;
    pub const LD_C_A: u8 = 0x4F;

    pub const LD_D_B: u8 = 0x50;
    pub const LD_D_C: u8 = 0x51;
    pub const LD_D_D: u8 = 0x52;
    pub const LD_D_E: u8 = 0x53;
    pub const LD_D_H: u8 = 0x54;
    pub const LD_D_L: u8 = 0x55;
    pub const LD_D_HL_IND: u8 = 0x56;
    pub const LD_D_A: u8 = 0x57;

    pub const LD_E_B: u8 = 0x58;
    pub const LD_E_C: u8 = 0x59;
    pub const LD_E_D: u8 = 0x5A;
    pub const LD_E_E: u8 = 0x5B;
    pub const LD_E_H: u8 = 0x5C;
    pub const LD_E_L: u8 = 0x5D;
    pub const LD_E_HL_IND: u8 = 0x5E;
    pub const LD_E_A: u8 = 0x5F;

    pub const LD_H_B: u8 = 0x60;
    pub const LD_H_C: u8 = 0x61;
    pub const LD_H_D: u8 = 0x62;
    pub const LD_H_E: u8 = 0x63;
    pub const LD_H_H: u8 = 0x64;
    pub const LD_H_L: u8 = 0x65;
    pub const LD_H_HL_IND: u8 = 0x66;
    pub const LD_H_A: u8 = 0x67;

    pub const LD_L_B: u8 = 0x68;
    pub const LD_L_C: u8 = 0x69;
    pub const LD_L_D: u8 = 0x6A;
    pub const LD_L_E: u8 = 0x6B;
    pub const LD_L_H: u8 = 0x6C;
    pub const LD_L_L: u8 = 0x6D;
    pub const LD_L_HL_IND: u8 = 0x6E;
    pub const LD_L_A: u8 = 0x6F;

    pub const LD_HL_IND_B: u8 = 0x70;
    pub const LD_HL_IND_C: u8 = 0x71;
    pub const LD_HL_IND_D: u8 = 0x72;
    pub const LD_HL_IND_E: u8 = 0x73;
    pub const LD_HL_IND_H: u8 = 0x74;
    pub const LD_HL_IND_L: u8 = 0x75;
    pub const HALT: u8 = 0x76;
    pub const LD_HL_IND_A: u8 = 0x77;

    pub const LD_A_B: u8 = 0x78;
    pub const LD_A_C: u8 = 0x79;
    pub const LD_A_D: u8 = 0x7A;
    pub const LD_A_E: u8 = 0x7B;
    pub const LD_A_H: u8 = 0x7C;
    pub const LD_A_L: u8 = 0x7D;
    pub const LD_A_HL_IND: u8 = 0x7E;
    pub const LD_A_A: u8 = 0x7F;

    pub const ADD_A_B: u8 = 0x80;
    pub const ADD_A_C: u8 = 0x81;
    pub const ADD_A_D: u8 = 0x82;
    pub const ADD_A_E: u8 = 0x83;
    pub const ADD_A_H: u8 = 0x84;
    pub const ADD_A_L: u8 = 0x85;
    pub const ADD_A_HL_IND: u8 = 0x86;
    pub const ADD_A_A: u8 = 0x87;

    pub const ADC_A_B: u8 = 0x88;
    pub const ADC_A_C: u8 = 0x89;
    pub const ADC_A_D: u8 = 0x8A;
    pub const ADC_A_E: u8 = 0x8B;
    pub const ADC_A_H: u8 = 0x8C;
    pub const ADC_A_L: u8 = 0x8D;
    pub const ADC_A_HL_IND: u8 = 0x8E;
    pub const ADC_A_A: u8 = 0x8F;

    pub const SUB_B: u8 = 0x90;
    pub const SUB_C: u8 = 0x91;
    pub const SUB_D: u8 = 0x92;
    pub const SUB_E: u8 = 0x93;
    pub const SUB_H: u8 = 0x94;
    pub const SUB_L: u8 = 0x95;
    pub const SUB_HL_IND: u8 = 0x96;
    pub const SUB_A: u8 = 0x97;

    pub const SBC_A_B: u8 = 0x98;
    pub const SBC_A_C: u8 = 0x99;
    pub const SBC_A_D: u8 = 0x9A;
    pub const SBC_A_E: u8 = 0x9B;
    pub const SBC_A_H: u8 = 0x9C;
    pub const SBC_A_L: u8 = 0x9D;
    pub const SBC_A_HL_IND: u8 = 0x9E;
    pub const SBC_A_A: u8 = 0x9F;

    pub const AND_B: u8 = 0xA0;
    pub const AND_C: u8 = 0xA1;
    pub const AND_D: u8 = 0xA2;
    pub const AND_E: u8 = 0xA3;
    pub const AND_H: u8 = 0xA4;
    pub const AND_L: u8 = 0xA5;
    pub const AND_HL_IND: u8 = 0xA6;
    pub const AND_A: u8 = 0xA7;

    pub const XOR_B: u8 = 0xA8;
    pub const XOR_C: u8 = 0xA9;
    pub const XOR_D: u8 = 0xAA;
    pub const XOR_E: u8 = 0xAB;
    pub const XOR_H: u8 = 0xAC;
    pub const XOR_L: u8 = 0xAD;
    pub const XOR_HL_IND: u8 = 0xAE;
    pub const XOR_A: u8 = 0xAF;

    pub const OR_B: u8 = 0xB0;
    pub const OR_C: u8 = 0xB1;
    pub const OR_D: u8 = 0xB2;
    pub const OR_E: u8 = 0xB3;
    pub const OR_H: u8 = 0xB4;
    pub const OR_L: u8 = 0xB5;
    pub const OR_HL_IND: u8 = 0xB6;
    pub const OR_A: u8 = 0xB7;

    pub const CP_B: u8 = 0xB8;
    pub const CP_C: u8 = 0xB9;
    pub const CP_D: u8 = 0xBA;
    pub const CP_E: u8 = 0xBB;
    pub const CP_H: u8 = 0xBC;
    pub const CP_L: u8 = 0xBD;
    pub const CP_HL_IND: u8 = 0xBE;
    pub const CP_A: u8 = 0xBF;

    pub const RET_NZ: u8 = 0xC0;
    pub const POP_BC: u8 = 0xC1;
    pub const JP_NZ_A16: u8 = 0xC2;
    pub const JP_A16: u8 = 0xC3;
    pub const CALL_NZ_A16: u8 = 0xC4;
    pub const PUSH_BC: u8 = 0xC5;
    pub const ADD_A_D8: u8 = 0xC6;
    pub const RST_00: u8 = 0xC7;
    pub const RET_Z: u8 = 0xC8;
    pub const RET: u8 = 0xC9;
    pub const JP_Z_A16: u8 = 0xCA;
    pub const CALL_Z_A16: u8 = 0xCC;
    pub const CALL_A16: u8 = 0xCD;
    pub const ADC_A_D8: u8 = 0xCE;
    pub const RST_08: u8 = 0xCF;

    pub const RET_NC: u8 = 0xD0;
    pub const POP_DE: u8 = 0xD1;
    pub const JP_NC_A16: u8 = 0xD2;
    pub const CALL_NC_A16: u8 = 0xD4;
    pub const PUSH_DE: u8 = 0xD5;
    pub const SUB_D8: u8 = 0xD6;
    pub const RST_10: u8 = 0xD7;
    pub const RET_C: u8 = 0xD8;
    pub const RETI: u8 = 0xD9;
    pub const JP_C_A16: u8 = 0xDA;
    pub const CALL_C_A16: u8 = 0xDC;
    pub const SBC_A_D8: u8 = 0xDE;
    pub const RST_18: u8 = 0xDF;

    pub const LDH_A8_IND_A: u8 = 0xE0;
    pub const POP_HL: u8 = 0xE1;
    pub const LD_C_IND_A: u8 = 0xE2;
    pub const PUSH_HL: u8 = 0xE5;
    pub const AND_D8: u8 = 0xE6;
    pub const RST_20: u8 = 0xE7;
    pub const ADD_SP_R8: u8 = 0xE8;
    pub const JP_HL: u8 = 0xE9;
    pub const LD_A16_IND_A: u8 = 0xEA;
    pub const XOR_D8: u8 = 0xEE;
    pub const RST_28: u8 = 0xEF;

    pub const LDH_A_IND_A8: u8 = 0xF0;
    pub const POP_AF: u8 = 0xF1;
    pub const LD_A_C_IND: u8 = 0xF2;
    pub const DI: u8 = 0xF3;
    pub const PUSH_AF: u8 = 0xF5;
    pub const OR_D8: u8 = 0xF6;
    pub const RST_30: u8 = 0xF7;
    pub const LD_HL_SP_R8: u8 = 0xF8;
    pub const LD_SP_HL: u8 = 0xF9;
    pub const LD_A_A16_IND: u8 = 0xFA;
    pub const EI: u8 = 0xFB;
    pub const CP_D8: u8 = 0xFE;
    pub const RST_38: u8 = 0xFF;
}

pub mod cb {
    pub const PREFIX: u8 = 0xCB;

    pub const RLC_B: u8 = 0x00;
    pub const RLC_C: u8 = 0x01;
    pub const RLC_D: u8 = 0x02;
    pub const RLC_E: u8 = 0x03;
    pub const RLC_H: u8 = 0x04;
    pub const RLC_L: u8 = 0x05;
    pub const RLC_HL_IND: u8 = 0x06;
    pub const RLC_A: u8 = 0x07;

    pub const RRC_B: u8 = 0x08;
    pub const RRC_C: u8 = 0x09;
    pub const RRC_D: u8 = 0x0A;
    pub const RRC_E: u8 = 0x0B;
    pub const RRC_H: u8 = 0x0C;
    pub const RRC_L: u8 = 0x0D;
    pub const RRC_HL_IND: u8 = 0x0E;
    pub const RRC_A: u8 = 0x0F;

    pub const RL_B: u8 = 0x10;
    pub const RL_C: u8 = 0x11;
    pub const RL_D: u8 = 0x12;
    pub const RL_E: u8 = 0x13;
    pub const RL_H: u8 = 0x14;
    pub const RL_L: u8 = 0x15;
    pub const RL_HL_IND: u8 = 0x16;
    pub const RL_A: u8 = 0x17;

    pub const RR_B: u8 = 0x18;
    pub const RR_C: u8 = 0x19;
    pub const RR_D: u8 = 0x1A;
    pub const RR_E: u8 = 0x1B;
    pub const RR_H: u8 = 0x1C;
    pub const RR_L: u8 = 0x1D;
    pub const RR_HL_IND: u8 = 0x1E;
    pub const RR_A: u8 = 0x1F;

    pub const SLA_B: u8 = 0x20;
    pub const SLA_C: u8 = 0x21;
    pub const SLA_D: u8 = 0x22;
    pub const SLA_E: u8 = 0x23;
    pub const SLA_H: u8 = 0x24;
    pub const SLA_L: u8 = 0x25;
    pub const SLA_HL_IND: u8 = 0x26;
    pub const SLA_A: u8 = 0x27;

    pub const SRA_B: u8 = 0x28;
    pub const SRA_C: u8 = 0x29;
    pub const SRA_D: u8 = 0x2A;
    pub const SRA_E: u8 = 0x2B;
    pub const SRA_H: u8 = 0x2C;
    pub const SRA_L: u8 = 0x2D;
    pub const SRA_HL_IND: u8 = 0x2E;
    pub const SRA_A: u8 = 0x2F;

    pub const SWAP_B: u8 = 0x30;
    pub const SWAP_C: u8 = 0x31;
    pub const SWAP_D: u8 = 0x32;
    pub const SWAP_E: u8 = 0x33;
    pub const SWAP_H: u8 = 0x34;
    pub const SWAP_L: u8 = 0x35;
    pub const SWAP_HL_IND: u8 = 0x36;
    pub const SWAP_A: u8 = 0x37;

    pub const SRL_B: u8 = 0x38;
    pub const SRL_C: u8 = 0x39;
    pub const SRL_D: u8 = 0x3A;
    pub const SRL_E: u8 = 0x3B;
    pub const SRL_H: u8 = 0x3C;
    pub const SRL_L: u8 = 0x3D;
    pub const SRL_HL_IND: u8 = 0x3E;
    pub const SRL_A: u8 = 0x3F;

    pub const BIT_0_B: u8 = 0x40;
    pub const BIT_0_C: u8 = 0x41;
    pub const BIT_0_D: u8 = 0x42;
    pub const BIT_0_E: u8 = 0x43;
    pub const BIT_0_H: u8 = 0x44;
    pub const BIT_0_L: u8 = 0x45;
    pub const BIT_0_HL_IND: u8 = 0x46;
    pub const BIT_0_A: u8 = 0x47;

    pub const BIT_1_B: u8 = 0x48;
    pub const BIT_1_C: u8 = 0x49;
    pub const BIT_1_D: u8 = 0x4A;
    pub const BIT_1_E: u8 = 0x4B;
    pub const BIT_1_H: u8 = 0x4C;
    pub const BIT_1_L: u8 = 0x4D;
    pub const BIT_1_HL_IND: u8 = 0x4E;
    pub const BIT_1_A: u8 = 0x4F;

    pub const BIT_2_B: u8 = 0x50;
    pub const BIT_2_C: u8 = 0x51;
    pub const BIT_2_D: u8 = 0x52;
    pub const BIT_2_E: u8 = 0x53;
    pub const BIT_2_H: u8 = 0x54;
    pub const BIT_2_L: u8 = 0x55;
    pub const BIT_2_HL_IND: u8 = 0x56;
    pub const BIT_2_A: u8 = 0x57;

    pub const BIT_3_B: u8 = 0x58;
    pub const BIT_3_C: u8 = 0x59;
    pub const BIT_3_D: u8 = 0x5A;
    pub const BIT_3_E: u8 = 0x5B;
    pub const BIT_3_H: u8 = 0x5C;
    pub const BIT_3_L: u8 = 0x5D;
    pub const BIT_3_HL_IND: u8 = 0x5E;
    pub const BIT_3_A: u8 = 0x5F;

    pub const BIT_4_B: u8 = 0x60;
    pub const BIT_4_C: u8 = 0x61;
    pub const BIT_4_D: u8 = 0x62;
    pub const BIT_4_E: u8 = 0x63;
    pub const BIT_4_H: u8 = 0x64;
    pub const BIT_4_L: u8 = 0x65;
    pub const BIT_4_HL_IND: u8 = 0x66;
    pub const BIT_4_A: u8 = 0x67;

    pub const BIT_5_B: u8 = 0x68;
    pub const BIT_5_C: u8 = 0x69;
    pub const BIT_5_D: u8 = 0x6A;
    pub const BIT_5_E: u8 = 0x6B;
    pub const BIT_5_H: u8 = 0x6C;
    pub const BIT_5_L: u8 = 0x6D;
    pub const BIT_5_HL_IND: u8 = 0x6E;
    pub const BIT_5_A: u8 = 0x6F;

    pub const BIT_6_B: u8 = 0x70;
    pub const BIT_6_C: u8 = 0x71;
    pub const BIT_6_D: u8 = 0x72;
    pub const BIT_6_E: u8 = 0x73;
    pub const BIT_6_H: u8 = 0x74;
    pub const BIT_6_L: u8 = 0x75;
    pub const BIT_6_HL_IND: u8 = 0x76;
    pub const BIT_6_A: u8 = 0x77;

    pub const BIT_7_B: u8 = 0x78;
    pub const BIT_7_C: u8 = 0x79;
    pub const BIT_7_D: u8 = 0x7A;
    pub const BIT_7_E: u8 = 0x7B;
    pub const BIT_7_H: u8 = 0x7C;
    pub const BIT_7_L: u8 = 0x7D;
    pub const BIT_7_HL_IND: u8 = 0x7E;
    pub const BIT_7_A: u8 = 0x7F;

    pub const RES_0_B: u8 = 0x80;
    pub const RES_0_C: u8 = 0x81;
    pub const RES_0_D: u8 = 0x82;
    pub const RES_0_E: u8 = 0x83;
    pub const RES_0_H: u8 = 0x84;
    pub const RES_0_L: u8 = 0x85;
    pub const RES_0_HL_IND: u8 = 0x86;
    pub const RES_0_A: u8 = 0x87;

    pub const RES_1_B: u8 = 0x88;
    pub const RES_1_C: u8 = 0x89;
    pub const RES_1_D: u8 = 0x8A;
    pub const RES_1_E: u8 = 0x8B;
    pub const RES_1_H: u8 = 0x8C;
    pub const RES_1_L: u8 = 0x8D;
    pub const RES_1_HL_IND: u8 = 0x8E;
    pub const RES_1_A: u8 = 0x8F;

    pub const RES_2_B: u8 = 0x90;
    pub const RES_2_C: u8 = 0x91;
    pub const RES_2_D: u8 = 0x92;
    pub const RES_2_E: u8 = 0x93;
    pub const RES_2_H: u8 = 0x94;
    pub const RES_2_L: u8 = 0x95;
    pub const RES_2_HL_IND: u8 = 0x96;
    pub const RES_2_A: u8 = 0x97;

    pub const RES_3_B: u8 = 0x98;
    pub const RES_3_C: u8 = 0x99;
    pub const RES_3_D: u8 = 0x9A;
    pub const RES_3_E: u8 = 0x9B;
    pub const RES_3_H: u8 = 0x9C;
    pub const RES_3_L: u8 = 0x9D;
    pub const RES_3_HL_IND: u8 = 0x9E;
    pub const RES_3_A: u8 = 0x9F;

    pub const RES_4_B: u8 = 0xA0;
    pub const RES_4_C: u8 = 0xA1;
    pub const RES_4_D: u8 = 0xA2;
    pub const RES_4_E: u8 = 0xA3;
    pub const RES_4_H: u8 = 0xA4;
    pub const RES_4_L: u8 = 0xA5;
    pub const RES_4_HL_IND: u8 = 0xA6;
    pub const RES_4_A: u8 = 0xA7;

    pub const RES_5_B: u8 = 0xA8;
    pub const RES_5_C: u8 = 0xA9;
    pub const RES_5_D: u8 = 0xAA;
    pub const RES_5_E: u8 = 0xAB;
    pub const RES_5_H: u8 = 0xAC;
    pub const RES_5_L: u8 = 0xAD;
    pub const RES_5_HL_IND: u8 = 0xAE;
    pub const RES_5_A: u8 = 0xAF;

    pub const RES_6_B: u8 = 0xB0;
    pub const RES_6_C: u8 = 0xB1;
    pub const RES_6_D: u8 = 0xB2;
    pub const RES_6_E: u8 = 0xB3;
    pub const RES_6_H: u8 = 0xB4;
    pub const RES_6_L: u8 = 0xB5;
    pub const RES_6_HL_IND: u8 = 0xB6;
    pub const RES_6_A: u8 = 0xB7;

    pub const RES_7_B: u8 = 0xB8;
    pub const RES_7_C: u8 = 0xB9;
    pub const RES_7_D: u8 = 0xBA;
    pub const RES_7_E: u8 = 0xBB;
    pub const RES_7_H: u8 = 0xBC;
    pub const RES_7_L: u8 = 0xBD;
    pub const RES_7_HL_IND: u8 = 0xBE;
    pub const RES_7_A: u8 = 0xBF;

    pub const SET_0_B: u8 = 0xC0;
    pub const SET_0_C: u8 = 0xC1;
    pub const SET_0_D: u8 = 0xC2;
    pub const SET_0_E: u8 = 0xC3;
    pub const SET_0_H: u8 = 0xC4;
    pub const SET_0_L: u8 = 0xC5;
    pub const SET_0_HL_IND: u8 = 0xC6;
    pub const SET_0_A: u8 = 0xC7;

    pub const SET_1_B: u8 = 0xC8;
    pub const SET_1_C: u8 = 0xC9;
    pub const SET_1_D: u8 = 0xCA;
    pub const SET_1_E: u8 = 0xCB;
    pub const SET_1_H: u8 = 0xCC;
    pub const SET_1_L: u8 = 0xCD;
    pub const SET_1_HL_IND: u8 = 0xCE;
    pub const SET_1_A: u8 = 0xCF;

    pub const SET_2_B: u8 = 0xD0;
    pub const SET_2_C: u8 = 0xD1;
    pub const SET_2_D: u8 = 0xD2;
    pub const SET_2_E: u8 = 0xD3;
    pub const SET_2_H: u8 = 0xD4;
    pub const SET_2_L: u8 = 0xD5;
    pub const SET_2_HL_IND: u8 = 0xD6;
    pub const SET_2_A: u8 = 0xD7;

    pub const SET_3_B: u8 = 0xD8;
    pub const SET_3_C: u8 = 0xD9;
    pub const SET_3_D: u8 = 0xDA;
    pub const SET_3_E: u8 = 0xDB;
    pub const SET_3_H: u8 = 0xDC;
    pub const SET_3_L: u8 = 0xDD;
    pub const SET_3_HL_IND: u8 = 0xDE;
    pub const SET_3_A: u8 = 0xDF;

    pub const SET_4_B: u8 = 0xE0;
    pub const SET_4_C: u8 = 0xE1;
    pub const SET_4_D: u8 = 0xE2;
    pub const SET_4_E: u8 = 0xE3;
    pub const SET_4_H: u8 = 0xE4;
    pub const SET_4_L: u8 = 0xE5;
    pub const SET_4_HL_IND: u8 = 0xE6;
    pub const SET_4_A: u8 = 0xE7;

    pub const SET_5_B: u8 = 0xE8;
    pub const SET_5_C: u8 = 0xE9;
    pub const SET_5_D: u8 = 0xEA;
    pub const SET_5_E: u8 = 0xEB;
    pub const SET_5_H: u8 = 0xEC;
    pub const SET_5_L: u8 = 0xED;
    pub const SET_5_HL_IND: u8 = 0xEE;
    pub const SET_5_A: u8 = 0xEF;

    pub const SET_6_B: u8 = 0xF0;
    pub const SET_6_C: u8 = 0xF1;
    pub const SET_6_D: u8 = 0xF2;
    pub const SET_6_E: u8 = 0xF3;
    pub const SET_6_H: u8 = 0xF4;
    pub const SET_6_L: u8 = 0xF5;
    pub const SET_6_HL_IND: u8 = 0xF6;
    pub const SET_6_A: u8 = 0xF7;

    pub const SET_7_B: u8 = 0xF8;
    pub const SET_7_C: u8 = 0xF9;
    pub const SET_7_D: u8 = 0xFA;
    pub const SET_7_E: u8 = 0xFB;
    pub const SET_7_H: u8 = 0xFC;
    pub const SET_7_L: u8 = 0xFD;
    pub const SET_7_HL_IND: u8 = 0xFE;
    pub const SET_7_A: u8 = 0xFF;
}
