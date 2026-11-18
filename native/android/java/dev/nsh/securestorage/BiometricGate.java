package dev.nsh.securestorage;

import android.content.Context;
import android.content.DialogInterface;
import android.hardware.biometrics.BiometricPrompt;
import android.os.Build;
import android.os.CancellationSignal;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Function;

/**
 * Shows the system biometric prompt and waits for its one outcome. The Rust side loads this class
 * from a dex, because JNI cannot subclass {@link BiometricPrompt.AuthenticationCallback}. The
 * argument of {@link #apply} is {@code Object[] {Context, title, subtitle, negative, authenticators,
 * confirm}}, with the last two as decimal text and "true" or "false". The result is an Integer.
 */
public final class BiometricGate implements Function<Object, Object> {
    /** The prompt succeeded. */
    static final int OK = 0;
    /** The user cancelled, pressed the negative button, or the prompt timed out. */
    static final int CANCELLED = 1;
    /** Authentication failed for good: too many attempts, or a lockout. */
    static final int FAILED = 2;
    /** No biometric hardware, nothing enrolled, or no device credential. */
    static final int UNAVAILABLE = 3;
    /** Anything else. */
    static final int ERROR = 4;

    private static final long TIMEOUT_SECONDS = 120;
    private static final int DEVICE_CREDENTIAL = 0x8000;

    /** Collects the first outcome. Later callbacks are ignored, so a call resolves once. */
    private static final class Outcome extends BiometricPrompt.AuthenticationCallback {
        private final AtomicBoolean done = new AtomicBoolean(false);
        private final AtomicInteger result = new AtomicInteger(ERROR);
        private final CountDownLatch latch = new CountDownLatch(1);

        void finish(int code) {
            if (done.compareAndSet(false, true)) {
                result.set(code);
                latch.countDown();
            }
        }

        @Override
        public void onAuthenticationSucceeded(BiometricPrompt.AuthenticationResult authentication) {
            finish(OK);
        }

        @Override
        public void onAuthenticationError(int errorCode, CharSequence message) {
            finish(classify(errorCode));
        }
    }

    /** Maps a BiometricPrompt error code to an outcome. */
    static int classify(int errorCode) {
        switch (errorCode) {
            case 5: // BIOMETRIC_ERROR_CANCELED
            case 10: // BIOMETRIC_ERROR_USER_CANCELED
            case 13: // BIOMETRIC_ERROR_NEGATIVE_BUTTON
            case 3: // BIOMETRIC_ERROR_TIMEOUT
                return CANCELLED;
            case 7: // BIOMETRIC_ERROR_LOCKOUT
            case 9: // BIOMETRIC_ERROR_LOCKOUT_PERMANENT
                return FAILED;
            case 1: // BIOMETRIC_ERROR_HW_UNAVAILABLE
            case 11: // BIOMETRIC_ERROR_NO_BIOMETRICS
            case 12: // BIOMETRIC_ERROR_HW_NOT_PRESENT
            case 14: // BIOMETRIC_ERROR_NO_DEVICE_CREDENTIAL
                return UNAVAILABLE;
            default:
                return ERROR;
        }
    }

    @Override
    public Object apply(Object input) {
        try {
            Object[] args = (Object[]) input;
            return Integer.valueOf(
                    run(
                            (Context) args[0],
                            (String) args[1],
                            (String) args[2],
                            (String) args[3],
                            Integer.parseInt((String) args[4]),
                            Boolean.parseBoolean((String) args[5])));
        } catch (Throwable error) {
            return Integer.valueOf(ERROR);
        }
    }

    private static int run(
            Context context,
            String title,
            String subtitle,
            String negative,
            int authenticators,
            boolean confirmation)
            throws InterruptedException {
        final Outcome outcome = new Outcome();
        Executor executor = context.getMainExecutor();
        BiometricPrompt.Builder builder =
                new BiometricPrompt.Builder(context).setTitle(title).setSubtitle(subtitle);
        if (Build.VERSION.SDK_INT >= 29) {
            builder.setConfirmationRequired(confirmation);
        }
        boolean credential = (authenticators & DEVICE_CREDENTIAL) != 0;
        if (Build.VERSION.SDK_INT >= 30) {
            builder.setAllowedAuthenticators(authenticators);
        }
        // A prompt that accepts the device credential has its own fallback button and refuses a negative one.
        if (!credential || Build.VERSION.SDK_INT < 30) {
            builder.setNegativeButton(
                    negative,
                    executor,
                    new DialogInterface.OnClickListener() {
                        @Override
                        public void onClick(DialogInterface dialog, int which) {
                            outcome.finish(CANCELLED);
                        }
                    });
        }
        CancellationSignal signal = new CancellationSignal();
        builder.build().authenticate(signal, executor, outcome);
        if (!outcome.latch.await(TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
            signal.cancel();
            outcome.finish(CANCELLED);
        }
        return outcome.result.get();
    }
}
