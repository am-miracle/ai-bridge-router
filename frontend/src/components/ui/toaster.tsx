import { Toaster as HotToaster } from "react-hot-toast";

export function Toaster() {
  return (
    <HotToaster
      position="top-right"
      reverseOrder={false}
      gutter={8}
      toastOptions={{
        // Default options
        style: {
          background: "var(--card)",
          color: "var(--card-foreground)",
          borderRadius: "0.5rem",
          fontSize: "0.875rem",
          padding: "0.75rem 1rem",
          boxShadow: "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
        },
        // // Success toast
        success: {
          iconTheme: {
            primary: "var(--primary)",
            secondary: "var(--primary-foreground)",
          },
        },
        // Error toast
        error: {
          duration: 6000,
          iconTheme: {
            primary: "var(--destructive)",
            secondary: "var(--destructive-foreground)",
          },
        },
        // // Loading toast
        loading: {
          iconTheme: {
            primary: "var(--muted-foreground)",
            secondary: "var(--muted)",
          },
        },
      }}
    />
  );
}
