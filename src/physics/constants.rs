/* tep/physics/constants.rs */

/** Números físico-químicos do Tennessee Eastman Process — o que ESTE crate contribui pra
`monjolo::chemistry::Coefficients` (as fórmulas em si moram lá, ver `monjolo/chemistry.rs`; aqui só
os valores, extraídos diretamente do TEINIT em `teprob.f`, que nunca mudam durante a simulação).
Indexação: componentes A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7.

Newtype (não um `type TepConstants = Coefficients<8>` alias direto) porque o mecanismo de campo
"comum" de `#[dynamic_model]` (`constants: TepConstants` em `reactor.rs` etc., sem `#[offer]`/
`#[need]`) inicializa via `Default::default()` — e as regras de orphan do Rust proíbem
`impl Default for monjolo::chemistry::Coefficients<8>` a partir daqui (nem o trait nem o tipo
genérico são locais a este crate). `Deref<Target = Coefficients<8>>` deixa `self.constants.avp[i]`
e `mixture_enthalpy(&z, t, ity, &self.constants)` funcionando exatamente como antes, sem mudar
nenhum site de chamada — só o `impl Default` mora aqui, não o `impl Deref` sendo contornado.
*/
pub struct TepConstants(monjolo::chemistry::Coefficients<8>);

impl TepConstants {
    /** Inicializa todas as constantes com os valores do TEINIT (teprob.f) */
    pub fn new() -> Self {
        Self(monjolo::chemistry::Coefficients {
            /* --- Massas molares --- */
            xmw: [2.0, 25.4, 28.0, 32.0, 46.0, 48.0, 62.0, 76.0],

            /* --- Pressão de vapor (Antoine) — A, B, C são gases não condensáveis → sem equação de
            Antoine ---
            */
            avp: [0.0, 0.0, 0.0, 15.92, 16.35, 16.35, 16.43, 17.21],
            bvp: [0.0, 0.0, 0.0, -1444.0, -2114.0, -2114.0, -2748.0, -3318.0],
            cvp: [0.0, 0.0, 0.0, 259.0, 265.5, 265.5, 232.9, 249.6],

            /* --- Densidade líquida --- */
            ad: [1.0, 1.0, 1.0, 23.3, 33.9, 32.8, 49.9, 50.5],
            bd: [0.0, 0.0, 0.0, -0.0700, -0.0957, -0.0995, -0.0191, -0.0541],
            cd: [0.0, 0.0, 0.0, -0.0002, -0.000152, -0.000233, -0.000425, -0.000150],

            /* --- Entalpia líquida --- */
            ah: [1.0e-6, 1.0e-6, 1.0e-6, 0.960e-6, 0.573e-6, 0.652e-6, 0.515e-6, 0.471e-6],
            bh: [0.0, 0.0, 0.0, 8.70e-9, 2.41e-9, 2.18e-9, 5.65e-10, 8.70e-10],
            ch: [0.0, 0.0, 0.0, 4.81e-11, 1.82e-11, 1.94e-11, 3.82e-12, 2.62e-12],

            /* --- Entalpia vapor --- */
            ag: [3.411e-6, 0.3799e-6, 0.2491e-6, 0.3567e-6, 0.3463e-6, 0.3930e-6, 0.170e-6, 0.150e-6],
            bg: [7.18e-10, 1.08e-9, 1.36e-11, 8.51e-10, 8.96e-10, 1.02e-9, 0.0, 0.0],
            cg: [6.0e-13, -3.98e-13, -3.93e-14, -3.12e-13, -3.27e-13, -3.12e-13, 0.0, 0.0],

            /* --- Entalpia de vaporização --- */
            av: [1.0e-6, 1.0e-6, 1.0e-6, 86.7e-6, 160.0e-6, 160.0e-6, 225.0e-6, 209.0e-6],
        })
    }
}

impl Default for TepConstants {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for TepConstants {
    type Target = monjolo::chemistry::Coefficients<8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
