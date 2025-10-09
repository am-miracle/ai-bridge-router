import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Zap, Shield, TrendingDown, Clock, CheckCircle2 } from "lucide-react";
import { BridgeIcon } from "./ui/icons";
import { motion } from "framer-motion";

const WELCOME_MODAL_KEY = "bridge-router-welcome-shown";

export function WelcomeModal() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    // Check if user has seen the welcome modal before
    const hasSeenWelcome = localStorage.getItem(WELCOME_MODAL_KEY);

    if (!hasSeenWelcome) {
      // Show modal after a short delay for better UX
      const timer = setTimeout(() => {
        setOpen(true);
      }, 500);

      return () => clearTimeout(timer);
    }
  }, []);

  const handleClose = () => {
    // Mark as seen in localStorage
    localStorage.setItem(WELCOME_MODAL_KEY, "true");
    setOpen(false);
  };

  const features = [
    {
      icon: <TrendingDown className="w-5 h-5 text-green-500" />,
      title: "Best Rates Guaranteed",
      description: "Compare quotes from 9+ bridges to find the lowest fees",
    },
    {
      icon: <Clock className="w-5 h-5 text-blue-500" />,
      title: "Fastest Routes",
      description: "See estimated transfer times and choose the quickest option",
    },
    {
      icon: <Shield className="w-5 h-5 text-purple-500" />,
      title: "Security Audits Visible",
      description: "View audit history and security scores for each bridge",
    },
    {
      icon: <Zap className="w-5 h-5 text-yellow-500" />,
      title: "All Bridges, One Place",
      description: "Access Across, Stargate, Wormhole, cBridge, and more",
    },
    {
      icon: <CheckCircle2 className="w-5 h-5 text-emerald-500" />,
      title: "No Hidden Fees",
      description: "Transparent pricing with detailed fee breakdowns including gas fees",
    },
  ];

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-center justify-center mb-2">
            <motion.div
              animate={{ rotate: [0, 9, -9, 0] }}
              transition={{
                duration: 3,
                repeat: Infinity,
                ease: "easeInOut",
              }}
              className="p-3 rounded-full bg-gradient-to-br from-blue-500 to-purple-600"
            >
              <BridgeIcon className="h-8 w-8 text-primary" />
            </motion.div>
          </div>
          <DialogTitle className="text-2xl text-center">Welcome to Bridge Router!</DialogTitle>
          <DialogDescription className="text-center text-base pt-2">
            Your one-stop solution for finding the best cross-chain bridge routes
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-6">
          <div className="space-y-3">
            {features.map((feature, index) => (
              <div
                key={index}
                className="flex items-start gap-4 p-4 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
              >
                <div className="flex-shrink-0 mt-0.5">{feature.icon}</div>
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-sm mb-1">{feature.title}</h3>
                  <p className="text-sm text-muted-foreground">{feature.description}</p>
                </div>
              </div>
            ))}
          </div>

          <div className="pt-4 pb-2">
            <div className="bg-muted/50 rounded-lg p-4 border-l-4 border-blue-500">
              <p className="text-sm text-muted-foreground">
                <span className="font-semibold text-foreground">Pro Tip:</span> Compare multiple
                bridges side-by-side to optimize for speed, cost, or security based on your needs!
              </p>
            </div>
          </div>
        </div>

        <div className="flex gap-3 pt-2">
          <Button variant="outline" onClick={handleClose} className="flex-1 border-0">
            Maybe Later
          </Button>
          <Button
            onClick={handleClose}
            className="flex-1 bg-gradient-to-r from-blue-500 to-purple-600 hover:from-blue-600 hover:to-purple-700"
          >
            Get Started
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
