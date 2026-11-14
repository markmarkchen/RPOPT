import threading

import numpy as np
import pytest

import rpopt
from conftest import NAMES, solved, theta_box

METHODS = {
    "sequential": lambda s: rpopt.SequentialSearch(s),
    "bst": lambda s: rpopt.BinarySearchTree(s),
    "multiway": lambda s: rpopt.MultiwayTree(s, 3),
    "linear_machine": lambda s: rpopt.LinearMachine(s),
    "descriptor": lambda s: rpopt.DescriptorFunction(s),
    "hash": lambda s: rpopt.HashTable(s),
    "hash_limit": lambda s: rpopt.HashTable.with_storage_limit(s, 1 << 20),
    "yann": lambda s: rpopt.Yann(s),
}


@pytest.mark.parametrize("name", NAMES)
def test_methods_agree(name):
    prog, sol = solved(name)
    lo, hi = theta_box(name)
    rng = np.random.default_rng(2)
    thetas = rng.uniform(lo, hi, size=(300, prog.num_t()))
    truth = np.array([sol.get_region_index(t) if sol.get_region_index(t) is not None else -1 for t in thetas])
    assert (truth >= 0).any()
    for mname, make in METHODS.items():
        try:
            loc = make(sol)
        except RuntimeError:
            # the linear machine can be infeasible for partitions that are not facet-to-facet
            assert mname == "linear_machine"
            continue
        many = loc.locate_many(thetas)
        assert many.dtype == np.int64 and many.shape == (len(thetas),)
        for k, th in enumerate(thetas):
            one = loc.locate(th)
            assert one == many[k]
            if one != truth[k]:
                # only allowed on a shared boundary: the answer must still contain the point
                assert one >= 0 and truth[k] >= 0, (mname, name, k)
                assert sol.critical_regions[one].is_inside(th, 1e-6), (mname, name, k)
        np.testing.assert_array_equal(loc.locate_many(thetas, parallel=False), many)
        assert loc.storage_floats() > 0


def test_outside_returns_minus_one():
    _, sol = solved("di_doc_N3")
    for make in METHODS.values():
        assert make(sol).locate(np.array([100.0, 100.0])) == -1


def test_wrong_dimension():
    _, sol = solved("di_doc_N3")
    loc = rpopt.BinarySearchTree(sol)
    with pytest.raises(ValueError):
        loc.locate(np.zeros(3))
    with pytest.raises(ValueError):
        loc.locate_many(np.zeros((4, 3)))


def test_hash_storage_limit():
    _, sol = solved("masses2_N4")
    small = rpopt.HashTable.with_storage_limit(sol, 1 << 20)
    assert small.storage_bytes() <= 1 << 20
    big = rpopt.HashTable.with_storage_limit(sol, 8 << 20)
    assert big.storage_bytes() <= 8 << 20


def test_gil_released():
    """A second Python thread keeps running while locate_many works."""
    prog, sol = solved("masses2_N6")
    loc = rpopt.BinarySearchTree(sol)
    lo, hi = theta_box("masses2_N6")
    thetas = np.random.default_rng(3).uniform(lo, hi, size=(2_000_000, prog.num_t()))
    ticks = []
    stop = threading.Event()

    def ticker():
        while not stop.is_set():
            ticks.append(1)
            stop.wait(0.001)

    t = threading.Thread(target=ticker)
    t.start()
    try:
        loc.locate_many(thetas, parallel=False)
    finally:
        stop.set()
        t.join()
    assert len(ticks) > 5
