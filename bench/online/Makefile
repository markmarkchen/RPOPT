# Builds online_bench against prebuilt DAQP and qpOASES static libraries (neither is vendored here).
#
#   DAQP:    https://github.com/darnstrom/daqp     (MIT)   -> $(DAQP_DIR)/include, $(DAQP_DIR)/build/libdaqp.a
#   qpOASES: https://github.com/coin-or/qpOASES    (LGPL-2.1, releases/3.2.1)
#            -> $(QPOASES_DIR)/include, $(QPOASES_DIR)/build/libqpOASES.a
#   (each library: compile src/*.c(pp) with -O3 -DNDEBUG and `ar rcs`; qpOASES also needs
#    -DLINUX -D__NO_COPYRIGHT__ -D__SUPPRESSANYOUTPUT__)
#
#   make DAQP_DIR=... QPOASES_DIR=...
#   cargo run --release --example export_online -- data ../problems/di_N6.json ...
#   ./online_bench data/*.bin

DAQP_DIR    ?= $(HOME)/.cache/online-solvers/daqp
QPOASES_DIR ?= $(HOME)/.cache/online-solvers/qpOASES
CXXFLAGS    ?= -O3 -DNDEBUG -std=c++17

online_bench: online_bench.cpp
	$(CXX) $(CXXFLAGS) -I$(DAQP_DIR)/include -I$(QPOASES_DIR)/include -DLINUX -D__NO_COPYRIGHT__ -D__SUPPRESSANYOUTPUT__ \
		$< $(QPOASES_DIR)/build/libqpOASES.a $(DAQP_DIR)/build/libdaqp.a -o $@

clean:
	rm -f online_bench
