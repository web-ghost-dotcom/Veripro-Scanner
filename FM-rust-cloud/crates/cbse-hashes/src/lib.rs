// SPDX-License-Identifier: AGPL-3.0

//! Hashing utilities for EVM execution

use keccak_hash::keccak;
use sha3::{Digest, Keccak256};
use xxhash_rust::xxh3::Xxh3;

/// Compute Keccak256 hash
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

/// Compute Keccak256 hash using keccak_hash crate
pub fn keccak_hash(data: &[u8]) -> [u8; 32] {
    keccak(data).0
}

/// Compute xxHash3 hash (fast, non-cryptographic)
pub fn xxhash3(data: &[u8]) -> u64 {
    let mut hasher = Xxh3::new();
    hasher.update(data);
    hasher.digest()
}

/// Compute function selector (first 4 bytes of keccak256)
pub fn function_selector(signature: &str) -> [u8; 4] {
    let hash = keccak256(signature.as_bytes());
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&hash[..4]);
    selector
}

/// Compute event topic (keccak256 of event signature)
pub fn event_topic(signature: &str) -> [u8; 32] {
    keccak256(signature.as_bytes())
}

/// Precomputed keccak256_256 lookup: maps keccak256(x) -> x for x in 0..256
/// This is used for symbolic execution optimizations
pub fn get_keccak256_256_preimage(hash: &[u8; 32]) -> Option<u8> {
    // Convert hash to hex string for matching
    let hash_hex = hex::encode(hash).to_uppercase();

    // Match against known preimages (first 32 for brevity, full table can be generated)
    match hash_hex.as_str() {
        "290DECD9548B62A8D60345A988386FC84BA6BC95484008F6362F93160EF3E563" => Some(0),
        "B10E2D527612073B26EECDFD717E6A320CF44B4AFAC2B0732D9FCBE2B7FA0CF6" => Some(1),
        "405787FA12A823E0F2B7631CC41B3BA8828B3321CA811111FA75CD3AA3BB5ACE" => Some(2),
        "C2575A0E9E593C00F959F8C92F12DB2869C3395A3B0502D05E2516446F71F85B" => Some(3),
        "8A35ACFBC15FF81A39AE7D344FD709F28E8600B4AA8C65C6B64BFE7FE36BD19B" => Some(4),
        "036B6384B5ECA791C62761152D0C79BB0604C104A5FB6F4EB0703F3154BB3DB0" => Some(5),
        "F652222313E28459528D920B65115C16C04F3EFC82AAEDC97BE59F3F377C0D3F" => Some(6),
        "A66CC928B5EDB82AF9BD49922954155AB7B0942694BEA4CE44661D9A8736C688" => Some(7),
        "F3F7A9FE364FAAB93B216DA50A3214154F22A0A2B415B23A84C8169E8B636EE3" => Some(8),
        "6E1540171B6C0C960B71A7020D9F60077F6AF931A8BBF590DA0223DACF75C7AF" => Some(9),
        "C65A7BB8D6351C1CF70C95A316CC6A92839C986682D98BC35F958F4883F9D2A8" => Some(10),
        "290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563" => Some(0),
        "b10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6" => Some(1),
        "405787fa12a823e0f2b7631cc41b3ba8828b3321ca811111fa75cd3aa3bb5ace" => Some(2),
        "c2575a0e9e593c00f959f8c92f12db2869c3395a3b0502d05e2516446f71f85b" => Some(3),
        "8a35acfbc15ff81a39ae7d344fd709f28e8600b4aa8c65c6b64bfe7fe36bd19b" => Some(4),
        "036b6384b5eca791c62761152d0c79bb0604c104a5fb6f4eb0703f3154bb3db0" => Some(5),
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d3f" => Some(6),
        "a66cc928b5edb82af9bd49922954155ab7b0942694bea4ce44661d9a8736c688" => Some(7),
        "f3f7a9fe364faab93b216da50a3214154f22a0a2b415b23a84c8169e8b636ee3" => Some(8),
        "6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af" => Some(9),
        "c65a7bb8d6351c1cf70c95a316cc6a92839c986682d98bc35f958f4883f9d2a8" => Some(10),
        "0175b7a638427703f0dbe7bb9bbf987a2551717b34e79f33b5b1008d1fa01db9" => Some(11),
        "df6966c971051c3d54ec59162606531493a51404a002842f56009d7e5cf4a8c7" => Some(12),
        "d7b6990105719101dabeb77144f2a3385c8033acd3af97e9423a695e81ad1eb5" => Some(13),
        "bb7b4a454dc3493923482f07822329ed19e8244eff582cc204f8554c3620c3fd" => Some(14),
        "8d1108e10bcb7c27dddfc02ed9d693a074039d026cf4ea4240b40f7d581ac802" => Some(15),
        "1b6847dc741a1b0cd08d278845f9d819d87b734759afb55fe2de5cb82a9ae672" => Some(16),
        "31ecc21a745e3968a04e9570e4425bc18fa8019c68028196b546d1669c200c68" => Some(17),
        "bb8a6a4669ba250d26cd7a459eca9d215f8307e33aebe50379bc5a3617ec3444" => Some(18),
        "66de8ffda797e3de9c05e8fc57b3bf0ec28a930d40b0d285d93c06501cf6a090" => Some(19),
        "ce6d7b5282bd9a3661ae061feed1dbda4e52ab073b1f9285be6e155d9c38d4ec" => Some(20),
        "55f448fdea98c4d29eb340757ef0a66cd03dbb9538908a6a81d96026b71ec475" => Some(21),
        "d833147d7dc355ba459fc788f669e58cfaf9dc25ddcd0702e87d69c7b5124289" => Some(22),
        "c624b66cc0138b8fabc209247f72d758e1cf3343756d543badbf24212bed8c15" => Some(23),
        "b13d2d76d1f4b7be834882e410b3e3a8afaf69f83600ae24db354391d2378d2e" => Some(24),
        "944998273e477b495144fb8794c914197f3ccb46be2900f4698fd0ef743c9695" => Some(25),
        "057c384a7d1c54f3a1b2e5e67b2617b8224fdfd1ea7234eea573a6ff665ff63e" => Some(26),
        "3ad8aa4f87544323a9d1e5dd902f40c356527a7955687113db5f9a85ad579dc1" => Some(27),
        "0e4562a10381dec21b205ed72637e6b1b523bdd0e4d4d50af5cd23dd4500a211" => Some(28),
        "6d4407e7be21f808e6509aa9fa9143369579dd7d760fe20a2c09680fc146134f" => Some(29),
        "50bb669a95c7b50b7e8a6f09454034b2b14cf2b85c730dca9a539ca82cb6e350" => Some(30),
        "a03837a25210ee280c2113ff4b77ca23440b19d4866cca721c801278fd08d807" => Some(31),
        "c97bfaf2f8ee708c303a06d134f5ecd8389ae0432af62dc132a24118292866bb" => Some(32),
        "3a6357012c1a3ae0a17d304c9920310382d968ebcc4b1771f41c6b304205b570" => Some(33),
        "61035b26e3e9eee00e0d72fd1ee8ddca6894550dca6916ea2ac6baa90d11e510" => Some(34),
        "d57b2b5166478fd4318d2acc6cc2c704584312bdd8781b32d5d06abda57f4230" => Some(35),
        "7cd332d19b93bcabe3cce7ca0c18a052f57e5fd03b4758a09f30f5ddc4b22ec4" => Some(36),
        "401968ff42a154441da5f6c4c935ac46b8671f0e062baaa62a7545ba53bb6e4c" => Some(37),
        "744a2cf8fd7008e3d53b67916e73460df9fa5214e3ef23dd4259ca09493a3594" => Some(38),
        "98a476f1687bc3d60a2da2adbcba2c46958e61fa2fb4042cd7bc5816a710195b" => Some(39),
        "e16da923a2d88192e5070f37b4571d58682c0d66212ec634d495f33de3f77ab5" => Some(40),
        "cb7c14ce178f56e2e8d86ab33ebc0ae081ba8556a00cd122038841867181caac" => Some(41),
        "beced09521047d05b8960b7e7bcc1d1292cf3e4b2a6b63f48335cbde5f7545d2" => Some(42),
        "11c44e4875b74d31ff9fd779bf2566af7bd15b87fc985d01f5094b89e3669e4f" => Some(43),
        "7416c943b4a09859521022fd2e90eac0dd9026dad28fa317782a135f28a86091" => Some(44),
        "4a2cc91ee622da3bc833a54c37ffcb6f3ec23b7793efc5eaf5e71b7b406c5c06" => Some(45),
        "37fa166cbdbfbb1561ccd9ea985ec0218b5e68502e230525f544285b2bdf3d7e" => Some(46),
        "a813484aef6fb598f9f753daf162068ff39ccea4075cb95e1a30f86995b5b7ee" => Some(47),
        "6ff97a59c90d62cc7236ba3a37cd85351bf564556780cf8c1157a220f31f0cbb" => Some(48),
        "c54045fa7c6ec765e825df7f9e9bf9dec12c5cef146f93a5eee56772ee647fbc" => Some(49),
        "11df491316f14931039edfd4f8964c9a443b862f02d4c7611d18c2bc4e6ff697" => Some(50),
        "82a75bdeeae8604d839476ae9efd8b0e15aa447e21bfd7f41283bb54e22c9a82" => Some(51),
        "46bddb1178e94d7f2892ff5f366840eb658911794f2c3a44c450aa2c505186c1" => Some(52),
        "cfa4bec1d3298408bb5afcfcd9c430549c5b31f8aa5c5848151c0a55f473c34d" => Some(53),
        "4a11f94e20a93c79f6ec743a1954ec4fc2c08429ae2122118bf234b2185c81b8" => Some(54),
        "42a7b7dd785cd69714a189dffb3fd7d7174edc9ece837694ce50f7078f7c31ae" => Some(55),
        "38395c5dceade9603479b177b68959049485df8aa97b39f3533039af5f456199" => Some(56),
        "dc16fef70f8d5ddbc01ee3d903d1e69c18a3c7be080eb86a81e0578814ee58d3" => Some(57),
        "a2999d817b6757290b50e8ecf3fa939673403dd35c97de392fdb343b4015ce9e" => Some(58),
        "bbe3212124853f8b0084a66a2d057c2966e251e132af3691db153ab65f0d1a4d" => Some(59),
        "c6bb06cb7f92603de181bf256cd16846b93b752a170ff24824098b31aa008a7e" => Some(60),
        "ece66cfdbd22e3f37d348a3d8e19074452862cd65fd4b9a11f0336d1ac6d1dc3" => Some(61),
        "8d800d6614d35eed73733ee453164a3b48076eb3138f466adeeb9dec7bb31f70" => Some(62),
        "c03004e3ce0784bf68186394306849f9b7b1200073105cd9aeb554a1802b58fd" => Some(63),
        "352feee0eea125f11f791c1b77524172e9bc20f1b719b6cef0fc24f64db8e15e" => Some(64),
        "7c9785e8241615bc80415d89775984a1337d15dc1bf4ce50f41988b2a2b336a7" => Some(65),
        "38dfe4635b27babeca8be38d3b448cb5161a639b899a14825ba9c8d7892eb8c3" => Some(66),
        "9690ad99d6ce244efa8a0f6c2d04036d3b33a9474db32a71b71135c695102793" => Some(67),
        "9b22d3d61959b4d3528b1d8ba932c96fbe302b36a1aad1d95cab54f9e0a135ea" => Some(68),
        "a80a8fcc11760162f08bb091d2c9389d07f2b73d0e996161dfac6f1043b5fc0b" => Some(69),
        "128667f541fed74a8429f9d592c26c2c6a4beb9ae5ead9912c98b2595c842310" => Some(70),
        "c43c1e24e1884c4e28a16bbd9506f60b5ca9f18fc90635e729d3cfe13abcf001" => Some(71),
        "15040156076f78057c0a886f6dbac29221fa3c2646adbc8effedab98152ff32b" => Some(72),
        "37e472f504e93744df80d87316862f9a8fd41a7bc266c723bf77df7866d75f55" => Some(73),
        "fcc5ba1a98fc477b8948a04d08c6f4a76181fe75021370ab5e6abd22b1792a2a" => Some(74),
        "17b0af156a929edf60c351f3df2d53ed643fdd750aef9eda90dc7c8759a104a8" => Some(75),
        "42859d4f253f4d4a28ee9a59f9c9683a9404da2c5d329c733ab84f150db798a8" => Some(76),
        "1b524e1c8b5382bb913d0a2aae8ad83bb92a45fcb47761fa4a12f5b6316c2b20" => Some(77),
        "9b65e484ce3d961a557081a44c6c68a0a27eca0b88fce820bdd99c3dc223dcc7" => Some(78),
        "a2e8f972dc9f7d0b76177bb8be102e6bec069ee42c61080745e8825470e80c6c" => Some(79),
        "5529612556959ef813dbe8d0ed29336ab75e80a9b7855030760b2917b01e568a" => Some(80),
        "994a4b4eddb300691ee19901712848b1114bad8a1a4ae195e5abe0ec38021b94" => Some(81),
        "a9144a5e7efd259b8b0d55467f4696ed47ec83317d61501b76366dbcca65ce73" => Some(82),
        "4c83efb3982afbd500ab7c66d02b996df5fdc3d20660e61600390aad6d5f7f1e" => Some(83),
        "f0d642dbc7517672e217238a2f008f4f8cdad0586d8ce5113e9e09dcc6860619" => Some(84),
        "71beda120aafdd3bb922b360a066d10b7ce81d7ac2ad9874daac46e2282f6b45" => Some(85),
        "ea7419f5ae821e7204864e6a0871433ba612011908963bb42a64f42d65ad2f72" => Some(86),
        "e8e5595d268aaa85b36c3557e9d96c14a4fffaee9f45bcae0c407968a7109630" => Some(87),
        "657000d47e971dcfb21375bcfa3496f47a2a2f0f12c8aeb78a008ace6ae55ca5" => Some(88),
        "d73956b9e00d8f8bc5e44f7184df1387cdd652e7726b8ccda3db4859e02f31bf" => Some(89),
        "e8c3abd4193a84ec8a3fff3eeb3ecbcbd0979e0c977ac1dee06c6e01a60aca1b" => Some(90),
        "fcebc02dd307dc58cd01b156d63c6948b8f3422055fac1d836349b01722e9c52" => Some(91),
        "ec0b854938343f85eb39a6648b9e449c2e4aee4dc9b4e96ab592f9f497d05138" => Some(92),
        "2619ec68b255542e3da68c054bfe0d7d0f27b7fdbefc8bbccdd23188fc71fe7f" => Some(93),
        "34d3c319f536deb74ed8f1f3205d9aefef7487c819e77d3351630820dbff1118" => Some(94),
        "cc7ee599e5d59fee88c83157bd897847c5911dc7d317b3175e0b085198349973" => Some(95),
        "41c7ae758795765c6664a5d39bf63841c71ff191e9189522bad8ebff5d4eca98" => Some(96),
        "f0ecb75dd1820844c57b6762233d4e26853b3a7b8157bbd9f41f280a0f1cee9b" => Some(97),
        "b912c5eb6319a4a6a83580b9611610bedb31614179330261bfd87a41347cae1c" => Some(98),
        "d86d8a3f7c82c89ed8e04140017aa108a0a1469249f92c8f022b9dbafa87b883" => Some(99),
        "26700e13983fefbd9cf16da2ed70fa5c6798ac55062a4803121a869731e308d2" => Some(100),
        "8ff97419363ffd7000167f130ef7168fbea05faf9251824ca5043f113cc6a7c7" => Some(101),
        "46501879b8ca8525e8c2fd519e2fbfcfa2ebea26501294aa02cbfcfb12e94354" => Some(102),
        "9787eeb91fe3101235e4a76063c7023ecb40f923f97916639c598592fa30d6ae" => Some(103),
        "a2153420d844928b4421650203c77babc8b33d7f2e7b450e2966db0c22097753" => Some(104),
        "7fb4302e8e91f9110a6554c2c0a24601252c2a42c2220ca988efcfe399914308" => Some(105),
        "116fea137db6e131133e7f2bab296045d8f41cc5607279db17b218cab0929a51" => Some(106),
        "bd43cb8ece8cd1863bcd6082d65c5b0d25665b1ce17980f0da43c0ed545f98b4" => Some(107),
        "2b4a51ab505fc96a0952efda2ba61bcd3078d4c02c39a186ec16f21883fbe016" => Some(108),
        "5006b838207c6a9ae9b84d68f467dd4bb5c305fbfb6b04eab8faaabeec1e18d8" => Some(109),
        "9930d9ff0dee0ef5ca2f7710ea66b8f84dd0f5f5351ecffe72b952cd9db7142a" => Some(110),
        "39f2babe526038520877fc7c33d81accf578af4a06c5fa6b0d038cae36e12711" => Some(111),
        "8f6b23ffa15f0465e3176e15ca644cf24f86dc1312fe715484e3c4aead5eb78b" => Some(112),
        "a1fcd19bfe8c32a61095b6bfbb2664842857e148fcbb5188386c8cd40348d5b6" => Some(113),
        "dffbd64cc7c1a7eb27984335d9416d51137a03d3fabec7141025c62663253fe1" => Some(114),
        "f79bde9ddd17963ebce6f7d021d60de7c2bd0db944d23c900c0c0e775f530052" => Some(115),
        "19a0b39aa25ac793b5f6e9a0534364cc0b3fd1ea9b651e79c7f50a59d48ef813" => Some(116),
        "9a8d93986a7b9e6294572ea6736696119c195c1a9f5eae642d3c5fcd44e49dea" => Some(117),
        "b5732705f5241370a28908c2fe1303cb223f03b90d857fd0573f003f79fefed4" => Some(118),
        "7901cb5addcae2d210a531c604a76a660d77039093bac314de0816a16392aff1" => Some(119),
        "8dc6fb69531d98d70dc0420e638d2dfd04e09e1ec783ede9aac77da9c5a0dac4" => Some(120),
        "957bbdc7fad0dec56e7c96af4a3ab63aa9daf934a52ffce891945b7fb622d791" => Some(121),
        "f0440771a29e57e18c66727944770b82cc77924aef333c927ce6bdd2cdb3ae03" => Some(122),
        "5569044719a1ec3b04d0afa9e7a5310c7c0473331d13dc9fafe143b2c4e8148a" => Some(123),
        "9222cbf5d0ddc505a6f2f04716e22c226cee16a955fef88c618922096dae2fd0" => Some(124),
        "a913c8ac5320dae1c4a00ff23343947ed0fdf88d251e9bd2a5519d3d6162d222" => Some(125),
        "0f2ada1f2dbae48ae468fe0cdb7bcda7d0cffee8545442e682273ba01a6203a7" => Some(126),
        "66925e85f1a4743fd8d60ba595ed74887b7caf321dd83b21e04d77c115383408" => Some(127),
        "59f3fb058c6bba7a4e76396639fc4dd21bd59163db798899cf56cef48b3c9ec9" => Some(128),
        "76fce494794d92ac286b20d6126fc49ecb9cca2fa94b5c726f6ec1109b891414" => Some(129),
        "b2244e644cfe16f72b654fbc48ff0fecec8fc59649ca8625094bebd9bd2e4035" => Some(130),
        "1397b88f412a83a7f1c0d834c533e486ff1f24f42a31819e91b624931060a863" => Some(131),
        "50250e93f8c73d2c1be015ec28e8cd2feb871efa71e955ad24477aafb09484fa" => Some(132),
        "dbdaec72d84124d8c7c57ae448f5a4e3eedb34dba437fdcbe6d26496b68afe87" => Some(133),
        "46b7ea84944250856a716737059479854246a026d947c13d5a0929bc8c1bc81d" => Some(134),
        "171ab08901be24769dbebedbdf7e0245486fbc64ab975cd431a39533032d5415" => Some(135),
        "7ef464cf5a521d70c933977510816a0355b91a50eca2778837fb82da8448ecf6" => Some(136),
        "5bfa74c743914028161ae645d300d90bbdc659f169ca1469ec86b4960f7266cb" => Some(137),
        "834355d35cbfbd33b2397e201af04b52bdd40b9b51275f279ea47e93547b631e" => Some(138),
        "7b6bb1e9d1b017ff82945596cf3cfb1a6cee971c1ebb16f2c6bd23c2d642728e" => Some(139),
        "5f2f2dca1d951c7429b52007f396328c64c25e226c1867318158f7f2cbdd40a9" => Some(140),
        "37a1be2a88dadcd0e6062f54ddcc01a03360ba61ca7784a744e757488bf8ceb2" => Some(141),
        "8edd81ff20324ea0cfe70c700ff4e9db7580d269b423d9f61470b370819cbd17" => Some(142),
        "337f7913db22d91ef425f82102bc8075ef67e23a2be359965ea316e78e1eff3f" => Some(143),
        "60b1e32550f9d5f25f9dd040e7a106b15d8eb282dd6b3e1914c73d8066896412" => Some(144),
        "cdae184edd6bf71c1fb62d6e6682fdb2032455c0e50143742135fbbe809bd793" => Some(145),
        "6e452848784197f00927d379e3db9e69a5131d2269f862bfcd05a0b38f6abf7f" => Some(146),
        "28da5ca8143bfa5e9f642e58e5e87bef0a2eb0c00bcd4efdd01050293f5fac91" => Some(147),
        "7047a3cc0a76edcee45792ca71527c753f6167484f14b94c4a3bd2997516725c" => Some(148),
        "947035e97d0f7e1937f791bc189f60c984ceaaa7a8494fc67f9f8f4de8ccf2c6" => Some(149),
        "6aa7ec8ac2a999a90ce6c78668dffe4e487e2576a97ca366ec81ecb335af90d0" => Some(150),
        "354a83ed9988f79f6038d4c7a7dadbad8af32f4ad6df893e0e5807a1b1944ff9" => Some(151),
        "2237a976fa961f5921fd19f2b03c925c725d77b20ce8f790c19709c03de4d814" => Some(152),
        "72a152ddfb8e864297c917af52ea6c1c68aead0fee1a62673fcc7e0c94979d00" => Some(153),
        "44da158ba27f9252712a74ff6a55c5d531f69609f1f6e7f17c4443a8e2089be4" => Some(154),
        "bba9db4cdbea0a37c207bbb83e20f828cd4441c49891101dc94fd20dc8efc349" => Some(155),
        "af85b9071dfafeac1409d3f1d19bafc9bc7c37974cde8df0ee6168f0086e539c" => Some(156),
        "d26e832454299e9fabb89e0e5fffdc046d4e14431bc1bf607ffb2e8a1ddecf7b" => Some(157),
        "cfe2a20ff701a1f3e14f63bd70d6c6bc6fba8172ec6d5a505cdab3927c0a9de6" => Some(158),
        "0bc14066c33013fe88f66e314e4cf150b0b2d4d6451a1a51dbbd1c27cd11de28" => Some(159),
        "78fdc8d422c49ced035a9edf18d00d3c6a8d81df210f3e5e448e045e77b41e88" => Some(160),
        "aadc37b8ba5645e62f4546802db221593a94729ccbfc5a97d01365a88f649878" => Some(161),
        "aaf4f58de99300cfadc4585755f376d5fa747d5bc561d5bd9d710de1f91bf42d" => Some(162),
        "60859188cffe297f44dde29f2d2865634621f26215049caeb304ccba566a8b17" => Some(163),
        "e434dc35da084cf8d7e8186688ea2dacb53db7003d427af3abf351bd9d0a4e8d" => Some(164),
        "b29a2b3b6f2ff1b765777a231725941da5072cc4fcc30ac4a2ce09706e8ddeff" => Some(165),
        "2da56674729343acc9933752c8c469a244252915242eb6d4c02d11ddd69164a1" => Some(166),
        "b68792697ed876af8b4858b316f5b54d81f6861191ad2950c1fde6c3dc7b3dea" => Some(167),
        "bee89403b5bf0e626c2f71adb366311c697013df53107181a963adc459ef4d99" => Some(168),
        "dc471888e6136f84c49e531e9c9240dc4e3fba66da9d3a49e2af6202133683e0" => Some(169),
        "550d3de95be0bd28a79c3eb4ea7f05692c60b0602e48b49461e703379b08a71a" => Some(170),
        "fc377260a69a39dd786235c89f4bcd5d9639157731cac38071a0508750eb115a" => Some(171),
        "0a0a1bcadd9f6a5539376fa82276e043ae3cb4499daaaf8136572ecb1f9f0d60" => Some(172),
        "0440fd76b4e685d17019b0eef836cea9994650028b99dddfb48be06fa4240aa6" => Some(173),
        "df5d400f265039450228fa547df2bee79e6a350daa43fba4bd328bc654824c64" => Some(174),
        "def993a65205231625280c5e3c23e44b263d0aa948fbc330055626b8ab25a5a1" => Some(175),
        "238ba8d02078544847438db7773730a25d584074eac94489bd8eb86ca267c937" => Some(176),
        "04cb44c80b6fbf8ceb1d80af688c9f7c0b2ab5bf4a964cabe37041f23b23f7a8" => Some(177),
        "bbf265bea1b905c854054a8dbe97fedcc06fa54306551423711231a4ad0610c9" => Some(178),
        "236f2840bfc5dc34b28742dd0b4c9defe8a4a5fa9592e49ceffb9ab51b7eb974" => Some(179),
        "1c5f5ac147ec2dee04d8ce29bdbebbc58f578e0e1392da66f352a62e5c09c503" => Some(180),
        "22b88d74a6b23be687aa96340c881253c2e9873c526eec7366dc5f733ada306a" => Some(181),
        "3ae797ceef265e3a4f9c1978c47c759eb34a32909251dee7276db339b17b3de3" => Some(182),
        "6a79cc294e25eb1a13381e9f3361ee96c47ee7ed00bf73abadb8f9664bffd0a7" => Some(183),
        "d91d691c894f8266e3f2d5e558ad2349d6783327a752a4949bc554f514e34988" => Some(184),
        "e35848a7c6477cfe9366ae64571069fd3a5ad752a460d28c5f73d438b5e432bf" => Some(185),
        "f3b9eb9e163af2088b11de0a369fb583f58f9440e0e5c70fce0c59909ecece8a" => Some(186),
        "28afdd85196b637a3c64ff1f53af1ad8de145cf652297ede1b38f2cbd6a4b4bf" => Some(187),
        "6f1f0041084f67ced174808484bd05851de94443d775585e9d86d4c2589dba59" => Some(188),
        "d344f074c815fded543cd5a29a47659de529cd0adb1c1fae6eda2d685d422bd8" => Some(189),
        "4082d8aa0be13ab143f55d600665a8ae7ef90ba09d57c38fa538a2604d7e9827" => Some(190),
        "b52cf138a3505dc3d3cd84a77912f4be1a33df2c3065d3e4cb37fb1d5d1b5072" => Some(191),
        "5e29e30c8ea9a89560281b90dbe96fe6f067a8acc0f164a71449bf0da7d58d7e" => Some(192),
        "a4c9b5d989fa12d608052e66dc5a37a431d679e93d0ed25572f97f67460bb157" => Some(193),
        "b93edcd1e74716ac76d71e26ce3491be20745375dcd4848d8f3b91a3f785dbb1" => Some(194),
        "6d918f650e2b4a9f360977c4447e6376eb632ec1f687ba963aa9983e90086594" => Some(195),
        "2bde9b0c0857aee2cffdea6b8723eaf59894499ec278c18f020edd3c2295e424" => Some(196),
        "bacdda17ed986c07f827229709e1ded99d4da917a5e7e7ec15816eaf2cacf54c" => Some(197),
        "cfc479828d8133d824a47fe26326d458b6b94134276b945404197f42411564c3" => Some(198),
        "c1d0558604082af4380f8af6e6df686f24c7438ca4f2a67c86a71ee7852601f9" => Some(199),
        "e71fac6fb785942cc6c6404a423f94f32a28ae66d69ff41494c38bfd4788b2f8" => Some(200),
        "66be4f155c5ef2ebd3772b228f2f00681e4ed5826cdb3b1943cc11ad15ad1d28" => Some(201),
        "42d72674974f694b5f5159593243114d38a5c39c89d6b62fee061ff523240ee1" => Some(202),
        "a7ce836d032b2bf62b7e2097a8e0a6d8aeb35405ad15271e96d3b0188a1d06fb" => Some(203),
        "47197230e1e4b29fc0bd84d7d78966c0925452aff72a2a121538b102457e9ebe" => Some(204),
        "83978b4c69c48dd978ab43fe30f077615294f938fb7f936d9eb340e51ea7db2e" => Some(205),
        "d36cd1c74ef8d7326d8021b776c18fb5a5724b7f7bc93c2f42e43e10ef27d12a" => Some(206),
        "acb8d954e2cfef495862221e91bd7523613cf8808827cb33edfe4904cc51bf29" => Some(207),
        "e89d44c8fd6a9bac8af33ce47f56337617d449bf7ff3956b618c646de829cbcb" => Some(208),
        "695fb3134ad82c3b8022bc5464edd0bcc9424ef672b52245dcb6ab2374327ce3" => Some(209),
        "f2192e1030363415d7b4fb0406540a0060e8e2fc8982f3f32289379e11fa6546" => Some(210),
        "915c3eb987b20e1af620c1403197bf687fb7f18513b3a73fde6e78c7072c41a6" => Some(211),
        "9780e26d96b1f2a9a18ef8fc72d589dbf03ef788137b64f43897e83a91e7feec" => Some(212),
        "51858de9989bf7441865ebdadbf7382c8838edbf830f5d86a9a51ac773676dd6" => Some(213),
        "e767803f8ecf1dee6bb0345811f7312cda556058b19db6389ad9ae3568643ddd" => Some(214),
        "8a012a6de2943a5aa4d77acf5e695d4456760a3f1f30a5d6dc2079599187a071" => Some(215),
        "5320ad99a619a90804cd2efe3a5cf0ac1ac5c41ad9ff2c61cf699efdad771096" => Some(216),
        "cc6782fd46dd71c5f512301ab049782450b4eaf79fdac5443d93d274d3916786" => Some(217),
        "b3d6e86317c38844915b053a0c35ff2fc103b684e96cef2918ab06844eb51aaf" => Some(218),
        "4c0d3471ead8ee99fbd8249e33f683e07c6cd6071fe102dd09617b2c353de430" => Some(219),
        "3162b0988d4210bff484413ed451d170a03887272177efc0b7d000f10abe9edf" => Some(220),
        "ac507b9f8bf86ad8bb770f71cd2b1992902ae0314d93fc0f2bb011d70e796226" => Some(221),
        "fae8130c0619f84b4b44f01b84806f04e82e536d70e05f2356977fa318aecc1a" => Some(222),
        "65e3d48fa860a761b461ce1274f0d562f3db9a6a57cf04d8c90d68f5670b6aea" => Some(223),
        "8b43726243eeaf8325404568abece3264b546cf9d88671f09c24c87045fccb4f" => Some(224),
        "3efdd7a884ff9e18c9e5711c185aa6c5e413b68f23197997da5b1665ca978f99" => Some(225),
        "26a62d79192c78c3891f38189368673110b88734c09ed7453515def7525e07d8" => Some(226),
        "37f6a7f96b945f2f9a9127ccb4a8552fcb6938e53fe8f046db8da238398093e9" => Some(227),
        "04e4a0bb093261ee16386dadcef9e2a83913f4e1899464891421d20c1bbff74d" => Some(228),
        "5625f7c930b8b40de87dc8e69145d83fd1d81c61b6c31fb7cfe69fac65b28642" => Some(229),
        "d31ddb47b5e8664717d3718acbd132396ff496fe337159c99410be8658408a27" => Some(230),
        "6cb0db1d7354dfb4a1464318006df0643cafe2002a86a29ff8560f900fef28a1" => Some(231),
        "53c8da29bfa275271df3f270296d5a7d61b57f8848c89b3f65f49e21340b7592" => Some(232),
        "ea6426b4b8d70caa8ece9a88fb0a9d4a6b817bb4a43ac6fbef64cb0e589129ee" => Some(233),
        "61c831beab28d67d1bb40b5ae1a11e2757fa842f031a2d0bc94a7867bc5d26c2" => Some(234),
        "0446c598f3355ed7d8a3b7e0b99f9299d15e956a97faae081a0b49d17024abd2" => Some(235),
        "e7dfac380f4a6ed3a03e62f813161eff828766fa014393558e075e9ceb77d549" => Some(236),
        "0504e0a132d2ef5ca5f2fe74fc64437205bc10f32d5f13d533bf552916a94d3f" => Some(237),
        "db444da68c84f0a9ce08609100b69b8f3d5672687e0ca13fa3c0ac9eb2bde5d2" => Some(238),
        "dd0dc620e7584674cb3dba490d2eba9e68eca0bef228ee569a4a64f6559056e9" => Some(239),
        "681483e2251cd5e2885507bb09f76bed3b99d3c377dd48396177647bfb4aafda" => Some(240),
        "c29b39917e4e60f0fee5b6871b30a38e50531d76d1b0837811bd6351b34854ec" => Some(241),
        "83d76afc3887c0b7edd14a1affa7554bed3345ba68ddcd2a3326c7eae97b80d8" => Some(242),
        "2f5553803273e8bb29d913cc31bab953051c59f3ba57a71cf5591563ca721405" => Some(243),
        "fc6a672327474e1387fcbce1814a1de376d8561fc138561441ac6e396089e062" => Some(244),
        "81630654dfb0fd282a37117995646cdde2cf8eefe9f3f96fdb12cfda88df6668" => Some(245),
        "ddf78cfa378b5e068a248edaf3abef23ea9e62c66f86f18cc5e695cd36c9809b" => Some(246),
        "e9944ebef6e5a24035a31a727e8ff6da7c372d99949c1224483b857f6401e346" => Some(247),
        "6120b123382f98f7efe66abe6a3a3445788a87e48d4e6991f37baadcac0bef95" => Some(248),
        "168c8166292b85070409830617e84bdd7e3518b38e5ac430dc35ed7d16b07a86" => Some(249),
        "d84f57f3ffa76cc18982da4353cc5991158ec5ae4f6a9109d1d7a0ae2cba77ed" => Some(250),
        "3e7257b7272bb46d49cd6019b04ddee20da7c0cb13f7c1ec3391291b2ccebabc" => Some(251),
        "371f36870d18f32a11fea0f144b021c8b407bb50f8e0267c711123f454b963c0" => Some(252),
        "9346ac6dd7de6b96975fec380d4d994c4c12e6a8897544f22915316cc6cca280" => Some(253),
        "54075df80ec1ae6ac9100e1fd0ebf3246c17f5c933137af392011f4c5f61513a" => Some(254),
        "e08ec2af2cfc251225e1968fd6ca21e4044f129bffa95bac3503be8bdb30a367" => Some(255),
        _ => None,
    }
}

/// Check if a hash has a known preimage in the keccak256_256 table
pub fn has_keccak256_256_preimage(hash: &[u8; 32]) -> bool {
    get_keccak256_256_preimage(hash).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let data = b"hello world";
        let hash = keccak256(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_function_selector() {
        // transfer(address,uint256)
        let selector = function_selector("transfer(address,uint256)");
        // Known selector for ERC20 transfer
        assert_eq!(selector, [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_xxhash3() {
        let data = b"test data";
        let hash = xxhash3(data);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_keccak256_256_preimage() {
        // Test keccak256(0)
        let hash_0 =
            hex::decode("290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563")
                .unwrap();
        let mut hash_0_array = [0u8; 32];
        hash_0_array.copy_from_slice(&hash_0);

        assert_eq!(get_keccak256_256_preimage(&hash_0_array), Some(0));
        assert!(has_keccak256_256_preimage(&hash_0_array));

        // Test keccak256(1)
        let hash_1 =
            hex::decode("b10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6")
                .unwrap();
        let mut hash_1_array = [0u8; 32];
        hash_1_array.copy_from_slice(&hash_1);

        assert_eq!(get_keccak256_256_preimage(&hash_1_array), Some(1));
    }

    #[test]
    fn test_keccak256_256_no_preimage() {
        // Random hash with no preimage
        let random_hash = [0xFF; 32];
        assert_eq!(get_keccak256_256_preimage(&random_hash), None);
        assert!(!has_keccak256_256_preimage(&random_hash));
    }
}
