// MIP-DD solver interface that runs a shell predicate instead of linking a
// solver: each trial problem is written as MPS and `shell.command FILE` is
// run; exit 0 means "still bad" (the bug reproduces), anything else "okay".
// Force-included into MIP-DD's src/bugger.cpp by bench/mipdd.sh.
#ifndef __BUGGER_INTERFACES_SHELLINTERFACE_HPP__
#define __BUGGER_INTERFACES_SHELLINTERFACE_HPP__

#include <cstdlib>
#include <fstream>
#include <unistd.h>
#include "bugger/data/BuggerRun.hpp"

namespace bugger
{
   struct ShellParameters
   {
      String command { };
      String trial { };
   };

   template <typename REAL>
   class ShellInterface : public SolverInterface<REAL>
   {
      const ShellParameters& parameters;

   public:
      ShellInterface(const Message& _msg, const ShellParameters& _parameters)
         : SolverInterface<REAL>(_msg), parameters(_parameters) { }

      void
      print_header( ) const override
      {
         this->msg.info("shell predicate: {}\n", parameters.command);
      }

      void
      doSetUp(SolverSettings& settings, const Problem<REAL>& problem, const Solution<REAL>& solution) override
      {
         this->adjustment = &settings;
         this->model = &problem;
         this->reference = &solution;
      }

      std::pair<char, SolverStatus>
      solve(const Vec<int>& passcodes) override
      {
         MpsWriter<REAL>::writeProb(parameters.trial, *this->model);
         String cmd = parameters.command + " '" + parameters.trial + "' > /dev/null 2>&1";
         // exit 0: the predicate holds, reported as a fail so MIP-DD keeps the reduction
         return { std::system(cmd.c_str()) == 0 ? SolverRetcode::OBJECTIVEFAIL : SolverRetcode::OKAY,
                  SolverStatus::kUnknown };
      }
   };

   template <typename REAL>
   class ShellFactory : public SolverFactory<REAL>
   {
      ShellParameters parameters { };

   public:
      void
      addParameters(ParameterSet& parameterset) override
      {
         parameterset.addParameter("shell.command", "predicate run on each trial MPS file (exit 0: bug reproduces)", parameters.command);
      }

      std::unique_ptr<SolverInterface<REAL>>
      create_solver(const Message& msg) override
      {
         if( parameters.trial.empty() )
         {
            const char* dir = std::getenv("TMPDIR");
            parameters.trial = String(dir ? dir : "/tmp") + "/mipdd_trial_" + std::to_string(getpid()) + ".mps";
         }
         return std::unique_ptr<SolverInterface<REAL>>( new ShellInterface<REAL>( msg, parameters ) );
      }
   };

   template <typename REAL>
   std::shared_ptr<SolverFactory<REAL>>
   load_solver_factory( )
   {
      return std::shared_ptr<SolverFactory<REAL>>( new ShellFactory<REAL>( ) );
   }

} // namespace bugger

#endif
