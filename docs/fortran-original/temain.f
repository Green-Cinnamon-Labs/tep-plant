C ============================================================================
C  Tennessee Eastman Process Control Test Problem
C
C  Authors:
C    James J. Downs
C    Ernest F. Vogel
C
C  Organization:
C    Process and Control Systems Engineering
C    Tennessee Eastman Company
C    Kingsport, TN, USA
C
C  References:
C    - AIChE Annual Meeting, 1990
C    - Computers & Chemical Engineering, Vol.17, No.3 (1993)
C
C  Purpose of this file:
C    Demonstration program showing how to:
C      - Initialize the Tennessee Eastman process
C      - Apply a simple controller
C      - Integrate the plant dynamics over time
C ============================================================================


C ============================================================================
C  COMMON BLOCKS
C ============================================================================

C --- Measurements and manipulated variables (plant I/O)
      DOUBLE PRECISION XMEAS, XMV
      COMMON /PV/ XMEAS(41), XMV(12)

C --- Disturbance flags
      INTEGER IDV
      COMMON /DVEC/ IDV(20)

C --- Controller parameters and internal state
      DOUBLE PRECISION SETPT, GAIN, TAUI, ERROLD, DELTAT
      COMMON /CTRL/ SETPT, GAIN, TAUI, ERROLD, DELTAT


C ============================================================================
C  MAIN PROGRAM
C ============================================================================

      INTEGER I, NN, NPTS
      DOUBLE PRECISION TIME, YY(50), YP(50)

C --- Number of differential equations (states)
C     The Tennessee Eastman plant has exactly 50 states
      NN = 50

C --- Number of integration steps
      NPTS = 1000

C --- Integration step size
C     1 second converted to hours
      DELTAT = 1.0 / 3600.0

C --- Initialize the process
C     (TEINIT sets TIME = 0 internally)
      CALL TEINIT(NN, TIME, YY, YP)

C ============================================================================
C  CONTROLLER SETUP
C ============================================================================

C --- Example control objective:
C     Increase stripper level setpoint by +15%
      SETPT  = XMEAS(15) + 15.0
      GAIN   = 2.0
      TAUI   = 5.0
      ERROLD = 0.0

C --- Example manipulated variable change:
C     Reactor cooling water flow
      XMV(10) = 38.0

C --- Disable all disturbances
      DO 100 I = 1, 20
         IDV(I) = 0
 100  CONTINUE


C ============================================================================
C  SIMULATION LOOP
C ============================================================================

      DO 1000 I = 1, NPTS

C --- Apply discrete-time control law
         CALL CONTRL

C --- Output selected measurements
         CALL OUTPUT

C --- Integrate plant dynamics one step
         CALL INTGTR(NN, TIME, DELTAT, YY, YP)

 1000 CONTINUE

      STOP
      END


C ============================================================================
C  SUBROUTINE: CONTRL
C  Purpose:
C    Discrete-time PI controller example
C    Controls stripper liquid level via underflow valve
C ============================================================================

      SUBROUTINE CONTRL

      DOUBLE PRECISION XMEAS, XMV
      COMMON /PV/ XMEAS(41), XMV(12)

      DOUBLE PRECISION SETPT, GAIN, TAUI, ERROLD, DELTAT
      COMMON /CTRL/ SETPT, GAIN, TAUI, ERROLD, DELTAT

      DOUBLE PRECISION ERR, DXMV

C --- Control error (setpoint - measurement)
      ERR = SETPT - XMEAS(15)

C --- PI controller (velocity form)
C     GAIN  = proportional gain
C     TAUI  = integral time (minutes)
      DXMV = GAIN * ( (ERR - ERROLD) + ERR * DELTAT * 60.0 / TAUI )

C --- Update manipulated variable:
C     Stripper liquid product flow (XMV(8))
      XMV(8) = XMV(8) - DXMV

      ERROLD = ERR

      RETURN
      END


C ============================================================================
C  SUBROUTINE: OUTPUT
C  Purpose:
C    Print selected plant variables to standard output
C ============================================================================

      SUBROUTINE OUTPUT

      DOUBLE PRECISION XMEAS, XMV
      COMMON /PV/ XMEAS(41), XMV(12)

      WRITE(6,100) XMEAS(9), XMEAS(15), XMV(8)

 100  FORMAT(1X,
     . 'Reac Temp = ', F6.2,
     . 2X, 'Stripper Lev = ', F6.2,
     . 2X, 'Stripper Underflow = ', F6.2)

      RETURN
      END


C ============================================================================
C  SUBROUTINE: INTGTR
C  Purpose:
C    Numerical integrator (explicit Euler method)
C
C  Notes:
C    - Calls TEFUNC to compute derivatives
C    - Advances state vector YY in time
C ============================================================================

      SUBROUTINE INTGTR(NN, TIME, DELTAT, YY, YP)

      INTEGER I, NN
      DOUBLE PRECISION TIME, DELTAT, YY(NN), YP(NN)

C --- Evaluate derivatives at current state
      CALL TEFUNC(NN, TIME, YY, YP)

C --- Advance simulation time
      TIME = TIME + DELTAT

C --- Euler integration step
      DO 100 I = 1, NN
         YY(I) = YY(I) + YP(I) * DELTAT
 100  CONTINUE

      RETURN
      END
