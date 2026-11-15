-- Formal verification of the mathematics behind RPOPT's `mplcp` and `mplp_lex` solvers.
-- See README.md for the status table and the Rust correspondence.
import RpoptProofs.Foundations.QP
import RpoptProofs.Foundations.LCP
import RpoptProofs.Tableau
import RpoptProofs.Pivot
import RpoptProofs.Lex
import RpoptProofs.Facet
import RpoptProofs.Algorithm2
import RpoptProofs.Coverage
import RpoptProofs.Optimizations.EntryFacetSkip
import RpoptProofs.MplpLex.Certificate
import RpoptProofs.MplpLex.Pivot
import RpoptProofs.MplpLex.LexUnique
