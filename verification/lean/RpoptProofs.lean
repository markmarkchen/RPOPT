-- Formal verification of the mathematics behind RPOPT's `mplcp` and `mplp_lex` solvers.
-- See README.md for the status table and the Rust correspondence.
import RpoptProofs.Foundations.QP
import RpoptProofs.Foundations.LCP
import RpoptProofs.Foundations.Chebyshev
import RpoptProofs.Foundations.Farkas
import RpoptProofs.Foundations.Twins
import RpoptProofs.Tableau
import RpoptProofs.Pivot
import RpoptProofs.Lex
import RpoptProofs.Facet
import RpoptProofs.Algorithm2
import RpoptProofs.Termination
import RpoptProofs.FacetWalk
import RpoptProofs.DependentFacet
import RpoptProofs.PivotMeasure
import RpoptProofs.Coverage
import RpoptProofs.Optimizations.EntryFacetSkip
import RpoptProofs.Optimizations.KnownRegionSkip
import RpoptProofs.Optimizations.ReverseCrossing
import RpoptProofs.Optimizations.BoxChain
import RpoptProofs.Optimizations.SeenClaims
import RpoptProofs.MplpLex.Certificate
import RpoptProofs.MplpLex.Pivot
import RpoptProofs.MplpLex.LexUnique
import RpoptProofs.MplpLex.Enumeration
import RpoptProofs.MplpLex.Crossing
