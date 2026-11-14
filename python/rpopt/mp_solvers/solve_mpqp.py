from enum import Enum

from .. import _rpopt


class mpqp_algorithm(Enum):
    """Algorithm selector, with PPOPT's member names (values are PPOPT's) plus RPOPT's extras."""

    combinatorial = "combinatorial"
    combinatorial_parallel = "p combinatorial"
    combinatorial_parallel_exp = "p combinatorial exp"
    graph = "graph"
    graph_exp = "graph exp"
    graph_parallel = "p graph"
    graph_parallel_exp = "p graph exp"
    combinatorial_graph = "combinatorial graph"
    geometric = "geometric"
    geometric_parallel = "p geometric"
    # RPOPT extras
    combinatorial_graph_parallel = "p combinatorial graph"
    ahmadi = "ahmadi"
    mplcp = "mplcp"
    mplcp_enum = "mplcp_enum"
    mplp_lex = "mplp_lex"


# PPOPT members that have no Rust counterpart are absent from this map.
_RUST_NAME = {
    "combinatorial": "combinatorial",
    "combinatorial_parallel": "combinatorial_parallel",
    "graph": "graph",
    "graph_exp": "graph_exp",
    "graph_parallel": "graph_parallel",
    "combinatorial_graph": "combinatorial_graph",
    "combinatorial_graph_parallel": "combinatorial_graph_parallel",
    "geometric": "geometric",
    "geometric_parallel": "geometric_parallel",
    "ahmadi": "ahmadi",
    "mplcp": "mplcp",
    "mplcp_enum": "mplcp_enum",
    "mplp_lex": "mplp_lex",
}


def solve_mpqp(problem, algorithm=mpqp_algorithm.combinatorial, seed=0, graph_samples=100):
    """Solve an MPLP_Program / MPQP_Program explicitly.

    :param algorithm: an ``mpqp_algorithm`` member or its name as a string (``"geometric"``).
    :param seed: seed of the random initialisation of graph / geometric / combinatorial_graph.
    :return: a Solution. The GIL is released while solving.
    """
    name = algorithm.name if isinstance(algorithm, mpqp_algorithm) else str(algorithm)
    if name not in _RUST_NAME:
        if name in mpqp_algorithm.__members__:
            raise NotImplementedError(f"RPOPT does not implement mpqp_algorithm.{name}")
        raise ValueError(f"unknown algorithm {name!r}; available: {sorted(_RUST_NAME)}")
    return _rpopt.solve(problem, _RUST_NAME[name], seed, graph_samples)
